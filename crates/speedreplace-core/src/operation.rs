use crate::encoding::{decode, encode};
use crate::engine::{build_hunks_for_text, transform};
use crate::{
    ApplyRequest, DiffHunk, Error, FileOperationStatus, OperationFileRecord, OperationManifest,
    OperationProgress, OperationStatus, ProgressStage, Result, UndoPreview, UndoRequest, io_error,
};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::NamedTempFile;
use uuid::Uuid;

pub fn apply(request: ApplyRequest) -> Result<OperationManifest> {
    apply_with_progress(request, &AtomicBool::new(false), |_| {})
}

pub fn apply_with_progress(
    request: ApplyRequest,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(OperationProgress),
) -> Result<OperationManifest> {
    let selected: HashSet<&str> = request.selected_paths.iter().map(String::as_str).collect();
    let apply_all = selected.is_empty();
    let candidates = request
        .scan
        .files
        .iter()
        .filter(|file| {
            file.changed
                && file.error.is_none()
                && (apply_all || selected.contains(file.path.as_str()))
        })
        .collect::<Vec<_>>();
    let total = candidates.len();
    let operation_id = Uuid::new_v4().to_string();
    let operation_dir = operation_dir(&request.backup_root, &operation_id)?;
    let objects_dir = operation_dir.join("objects");
    fs::create_dir_all(&objects_dir).map_err(|error| io_error(&objects_dir, error))?;
    let mut manifest = OperationManifest {
        schema_version: 2,
        id: operation_id,
        scan_id: request.scan.id.clone(),
        created_at: Utc::now().to_rfc3339(),
        status: OperationStatus::Running,
        finished_at: None,
        total_files: total,
        processed_files: 0,
        files: vec![],
    };
    write_manifest(&operation_dir, &manifest)?;

    for (index, preview) in candidates.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            finish_manifest(&operation_dir, &mut manifest, OperationStatus::Cancelled)?;
            return Err(Error::Cancelled);
        }
        progress(OperationProgress {
            stage: ProgressStage::Applying,
            completed: index,
            total,
            current_path: Some(preview.path.clone()),
        });
        let path = PathBuf::from(&preview.path);
        let selected_hunks = request.selected_hunks.get(&preview.path);
        let encoding_override = request
            .scan
            .request
            .encoding_overrides
            .get(&preview.path)
            .or(request.scan.request.default_encoding_override.as_ref())
            .map(String::as_str);
        let prepared = match prepare_one(
            &path,
            preview,
            &request.scan.request.rules,
            selected_hunks,
            encoding_override,
            &objects_dir,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                let record = OperationFileRecord {
                    path: preview.path.clone(),
                    source_hash: preview.source_hash.clone(),
                    result_hash: preview.result_hash.clone(),
                    backup_file: String::new(),
                    status: if matches!(error, Error::Conflict(_)) {
                        FileOperationStatus::Conflict
                    } else {
                        FileOperationStatus::Failed
                    },
                    message: Some(error.to_string()),
                };
                manifest.files.push(record);
                manifest.processed_files += 1;
                write_manifest(&operation_dir, &manifest)?;
                continue;
            }
        };
        manifest.files.push(prepared.record.clone());
        write_manifest(&operation_dir, &manifest)?;
        let commit_result = commit_prepared(&prepared);
        let record = manifest
            .files
            .last_mut()
            .expect("a pending record was just added");
        if let Err(error) = commit_result {
            reconcile_pending_record(&operation_dir, record, Some(error.to_string()))?;
            if record.status == FileOperationStatus::Skipped {
                record.status = FileOperationStatus::Failed;
            }
        } else {
            record.status = FileOperationStatus::Changed;
            record.message = None;
        }
        manifest.processed_files += 1;
        write_manifest(&operation_dir, &manifest)?;
    }
    finish_manifest(&operation_dir, &mut manifest, OperationStatus::Completed)?;
    progress(OperationProgress {
        stage: ProgressStage::Applying,
        completed: total,
        total,
        current_path: None,
    });
    if let Some(retain) = request.backup_retention {
        prune_operations(&request.backup_root, retain.max(1))?;
    }
    Ok(manifest)
}

struct PreparedChange {
    path: PathBuf,
    original: Vec<u8>,
    output: Vec<u8>,
    record: OperationFileRecord,
}

fn prepare_one(
    path: &Path,
    preview: &crate::FilePreview,
    rules: &[crate::ReplacementRule],
    selected_hunks: Option<&Vec<String>>,
    encoding_override: Option<&str>,
    objects_dir: &Path,
) -> Result<PreparedChange> {
    reject_symlink(path)?;
    let original = fs::read(path).map_err(|error| io_error(path, error))?;
    let current_hash = blake3::hash(&original).to_hex().to_string();
    if current_hash != preview.source_hash {
        return Err(Error::Conflict(preview.path.clone()));
    }
    let decoded = decode(&original, encoding_override)?;
    let full_result = transform(&decoded.text, rules)?.0;
    let full_output = encode(&full_result, &decoded)?;
    if blake3::hash(&full_output).to_hex().as_str() != preview.result_hash {
        return Err(Error::Conflict(format!(
            "preview result no longer matches for {}",
            preview.path
        )));
    }
    let after = if let Some(ids) = selected_hunks {
        let hunks = build_hunks_for_text(&decoded.text, &full_result);
        apply_selected_hunks(&decoded.text, &hunks, ids)?
    } else {
        full_result
    };
    let output = encode(&after, &decoded)?;
    let result_hash = blake3::hash(&output).to_hex().to_string();

    let backup = objects_dir.join(&preview.source_hash);
    Ok(PreparedChange {
        path: path.to_path_buf(),
        original,
        output,
        record: OperationFileRecord {
            path: preview.path.clone(),
            source_hash: preview.source_hash.clone(),
            result_hash,
            backup_file: backup.to_string_lossy().into_owned(),
            status: FileOperationStatus::Pending,
            message: None,
        },
    })
}

fn commit_prepared(prepared: &PreparedChange) -> Result<()> {
    let backup = Path::new(&prepared.record.backup_file);
    if !backup.exists() {
        fs::write(backup, &prepared.original).map_err(|error| io_error(backup, error))?;
    }
    let verified = fs::read(backup).map_err(|error| io_error(backup, error))?;
    if blake3::hash(&verified).to_hex().as_str() != prepared.record.source_hash {
        return Err(Error::Manifest(format!(
            "backup verification failed for {}",
            prepared.record.path
        )));
    }

    replace_file(&prepared.path, &prepared.output)?;
    let written = fs::read(&prepared.path).map_err(|error| io_error(&prepared.path, error))?;
    if blake3::hash(&written).to_hex().as_str() != prepared.record.result_hash {
        return Err(Error::Conflict(format!(
            "post-write verification failed for {}",
            prepared.record.path
        )));
    }
    Ok(())
}

fn apply_selected_hunks(
    source: &str,
    hunks: &[DiffHunk],
    selected_ids: &[String],
) -> Result<String> {
    let selected = selected_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if selected.is_empty()
        || selected
            .iter()
            .any(|id| !hunks.iter().any(|hunk| hunk.id == *id))
    {
        return Err(Error::Manifest("invalid or empty hunk selection".into()));
    }
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for hunk in hunks {
        let start = hunk.old_start.min(lines.len());
        let end = (hunk.old_start + hunk.old_len).min(lines.len());
        output.extend(lines[cursor..start].iter().copied());
        if selected.contains(hunk.id.as_str()) {
            output.push_str(&hunk.replacement);
        } else {
            output.extend(lines[start..end].iter().copied());
        }
        cursor = end;
    }
    output.extend(lines[cursor..].iter().copied());
    Ok(output)
}

pub fn prune_operations(backup_root: &str, retain: usize) -> Result<usize> {
    let manifests = list_operations(backup_root)?;
    let mut removed = 0;
    for manifest in manifests.into_iter().skip(retain) {
        let directory = operation_dir(backup_root, &manifest.id)?;
        fs::remove_dir_all(&directory).map_err(|error| io_error(&directory, error))?;
        removed += 1;
    }
    Ok(removed)
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Manifest("target has no parent".into()))?;
    let permissions = fs::metadata(path)
        .map_err(|error| io_error(path, error))?
        .permissions();
    let mut temp = NamedTempFile::new_in(parent).map_err(|error| io_error(parent, error))?;
    temp.write_all(bytes)
        .map_err(|error| io_error(temp.path(), error))?;
    temp.as_file()
        .sync_all()
        .map_err(|error| io_error(temp.path(), error))?;
    temp.as_file()
        .set_permissions(permissions)
        .map_err(|error| io_error(temp.path(), error))?;
    temp.persist(path)
        .map_err(|error| io_error(path, error.error))?;
    Ok(())
}

pub fn list_operations(backup_root: &str) -> Result<Vec<OperationManifest>> {
    read_operations(backup_root, false)
}

pub fn recover_interrupted_operations(backup_root: &str) -> Result<Vec<OperationManifest>> {
    read_operations(backup_root, true)
}

fn read_operations(backup_root: &str, recover: bool) -> Result<Vec<OperationManifest>> {
    let root = PathBuf::from(backup_root).join("operations");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut manifests = vec![];
    for entry in fs::read_dir(&root).map_err(|error| io_error(&root, error))? {
        let entry = entry.map_err(|error| io_error(&root, error))?;
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.is_file()
            && let Ok(mut manifest) = read_manifest(&manifest_path)
        {
            normalize_legacy_manifest(&mut manifest);
            if recover && manifest.status == OperationStatus::Running {
                recover_interrupted_manifest(&entry.path(), &mut manifest)?;
            }
            manifests.push(manifest);
        }
    }
    manifests.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(manifests)
}

pub fn undo_preview(backup_root: &str, operation_id: &str) -> Result<UndoPreview> {
    let manifest = load_operation(backup_root, operation_id)?;
    let mut preview = UndoPreview {
        operation_id: operation_id.to_owned(),
        safe: vec![],
        conflicts: vec![],
        missing: vec![],
    };
    for record in manifest
        .files
        .iter()
        .filter(|record| matches!(record.status, FileOperationStatus::Changed))
    {
        let path = Path::new(&record.path);
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                preview.conflicts.push(record.path.clone());
                continue;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                preview.missing.push(record.path.clone());
                continue;
            }
            Err(error) => return Err(io_error(path, error)),
        }
        let current = fs::read(path).map_err(|error| io_error(path, error))?;
        if blake3::hash(&current).to_hex().as_str() == record.result_hash {
            preview.safe.push(record.path.clone());
        } else {
            preview.conflicts.push(record.path.clone());
        }
    }
    Ok(preview)
}

pub fn undo(request: UndoRequest) -> Result<OperationManifest> {
    let mut manifest = load_operation(&request.backup_root, &request.operation_id)?;
    let operation_directory = operation_dir(&request.backup_root, &request.operation_id)?;
    let selected: HashSet<&str> = request.selected_paths.iter().map(String::as_str).collect();
    let restore_all = selected.is_empty();
    for record in &mut manifest.files {
        if !matches!(record.status, FileOperationStatus::Changed)
            || (!restore_all && !selected.contains(record.path.as_str()))
        {
            continue;
        }
        let path = Path::new(&record.path);
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(path, error)),
        };
        if metadata
            .as_ref()
            .is_some_and(|metadata| metadata.file_type().is_symlink())
        {
            record.status = FileOperationStatus::Conflict;
            record.message = Some("Symbolic-link target; restore skipped".into());
            continue;
        }
        let target_exists = metadata.is_some();
        let conflict = match metadata {
            Some(_) => {
                let current = fs::read(path).map_err(|error| io_error(path, error))?;
                blake3::hash(&current).to_hex().as_str() != record.result_hash
            }
            None => false,
        };
        if conflict && !request.overwrite_conflicts {
            record.status = FileOperationStatus::Conflict;
            record.message = Some("File changed after replacement; restore skipped".into());
            continue;
        }
        let backup = operation_directory
            .join("objects")
            .join(&record.source_hash);
        let original = fs::read(&backup).map_err(|error| io_error(&backup, error))?;
        if blake3::hash(&original).to_hex().as_str() != record.source_hash {
            record.status = FileOperationStatus::Failed;
            record.message = Some("Backup hash mismatch".into());
            continue;
        }
        if target_exists {
            replace_file(path, &original)?;
        } else {
            fs::write(path, &original).map_err(|error| io_error(path, error))?;
        }
        record.status = FileOperationStatus::Restored;
        record.message = None;
    }
    let directory = operation_dir(&request.backup_root, &request.operation_id)?;
    write_manifest(&directory, &manifest)?;
    Ok(manifest)
}

fn operation_dir(backup_root: &str, operation_id: &str) -> Result<PathBuf> {
    let valid_id =
        Uuid::parse_str(operation_id).is_ok_and(|id| id.hyphenated().to_string() == operation_id);
    if !valid_id {
        return Err(Error::Manifest("invalid operation ID".into()));
    }
    Ok(PathBuf::from(backup_root)
        .join("operations")
        .join(operation_id))
}

fn write_manifest(directory: &Path, manifest: &OperationManifest) -> Result<()> {
    fs::create_dir_all(directory).map_err(|error| io_error(directory, error))?;
    let bytes =
        serde_json::to_vec_pretty(manifest).map_err(|error| Error::Manifest(error.to_string()))?;
    let path = directory.join("manifest.json");
    let mut temp = NamedTempFile::new_in(directory).map_err(|error| io_error(directory, error))?;
    temp.write_all(&bytes)
        .map_err(|error| io_error(temp.path(), error))?;
    temp.as_file()
        .sync_all()
        .map_err(|error| io_error(temp.path(), error))?;
    temp.persist(&path)
        .map_err(|error| io_error(&path, error.error))?;
    Ok(())
}

fn finish_manifest(
    directory: &Path,
    manifest: &mut OperationManifest,
    status: OperationStatus,
) -> Result<()> {
    manifest.status = status;
    manifest.finished_at = Some(Utc::now().to_rfc3339());
    write_manifest(directory, manifest)
}

fn recover_interrupted_manifest(directory: &Path, manifest: &mut OperationManifest) -> Result<()> {
    for record in &mut manifest.files {
        if record.status == FileOperationStatus::Pending {
            reconcile_pending_record(
                directory,
                record,
                Some("Recovered after an interrupted operation".into()),
            )?;
        }
    }
    manifest.processed_files = manifest.files.len();
    finish_manifest(directory, manifest, OperationStatus::Interrupted)
}

fn reconcile_pending_record(
    operation_directory: &Path,
    record: &mut OperationFileRecord,
    context: Option<String>,
) -> Result<()> {
    let path = Path::new(&record.path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            record.status = FileOperationStatus::Conflict;
            record.message = Some(format_context(context, "Target file is missing"));
            return Ok(());
        }
        Err(error) => return Err(io_error(path, error)),
    };
    if metadata.file_type().is_symlink() {
        record.status = FileOperationStatus::Conflict;
        record.message = Some(format_context(context, "Target is now a symbolic link"));
        return Ok(());
    }

    let current = fs::read(path).map_err(|error| io_error(path, error))?;
    let current_hash = blake3::hash(&current).to_hex().to_string();
    if current_hash == record.source_hash {
        record.status = FileOperationStatus::Skipped;
        record.message = Some(format_context(context, "File was not changed"));
    } else if current_hash == record.result_hash {
        let backup = operation_directory
            .join("objects")
            .join(&record.source_hash);
        match fs::read(&backup) {
            Ok(original) if blake3::hash(&original).to_hex().as_str() == record.source_hash => {
                record.backup_file = backup.to_string_lossy().into_owned();
                record.status = FileOperationStatus::Changed;
                record.message = context;
            }
            Ok(_) => {
                record.status = FileOperationStatus::Conflict;
                record.message = Some(format_context(context, "Backup hash does not match"));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                record.status = FileOperationStatus::Conflict;
                record.message = Some(format_context(context, "Verified backup is missing"));
            }
            Err(error) => return Err(io_error(&backup, error)),
        }
    } else {
        record.status = FileOperationStatus::Conflict;
        record.message = Some(format_context(
            context,
            "File contents match neither the source nor replacement",
        ));
    }
    Ok(())
}

fn format_context(context: Option<String>, detail: &str) -> String {
    context.map_or_else(
        || detail.to_owned(),
        |context| format!("{context}: {detail}"),
    )
}

fn read_manifest(path: &Path) -> Result<OperationManifest> {
    let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
    serde_json::from_slice(&bytes).map_err(|error| Error::Manifest(error.to_string()))
}

fn normalize_legacy_manifest(manifest: &mut OperationManifest) {
    if manifest.schema_version == 1 {
        manifest.total_files = manifest.files.len();
        manifest.processed_files = manifest.files.len();
    }
}

fn load_operation(backup_root: &str, operation_id: &str) -> Result<OperationManifest> {
    let directory = operation_dir(backup_root, operation_id)?;
    let mut manifest = read_manifest(&directory.join("manifest.json"))?;
    if !matches!(manifest.schema_version, 1 | 2) || manifest.id != operation_id {
        return Err(Error::Manifest(
            "operation manifest identity or schema does not match".into(),
        ));
    }
    normalize_legacy_manifest(&mut manifest);
    if manifest.status == OperationStatus::Running {
        recover_interrupted_manifest(&directory, &mut manifest)?;
    }
    Ok(manifest)
}

fn is_symlink(path: &Path) -> Result<bool> {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .map_err(|error| io_error(path, error))
}

fn reject_symlink(path: &Path) -> Result<()> {
    if is_symlink(path)? {
        return Err(Error::Conflict(format!(
            "symbolic links are not replaced: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FilterSpec, ReplacementRule, RuleMode, ScanRequest, scan};

    fn request(path: &Path) -> ScanRequest {
        ScanRequest {
            targets: vec![path.to_string_lossy().into_owned()],
            filters: FilterSpec::default(),
            rules: vec![ReplacementRule {
                id: "one".into(),
                enabled: true,
                search: "old".into(),
                replacement: "new".into(),
                mode: RuleMode::Literal,
                case_sensitive: true,
                whole_word: false,
                multiline: true,
                description: String::new(),
            }],
            encoding_overrides: Default::default(),
            default_encoding_override: None,
        }
    }

    #[test]
    fn apply_and_undo_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, b"old text\r\n").unwrap();
        let scan = scan(request(&target)).unwrap();
        let manifest = apply(ApplyRequest {
            scan,
            selected_paths: vec![],
            selected_hunks: Default::default(),
            backup_retention: None,
            backup_root: directory
                .path()
                .join("state")
                .to_string_lossy()
                .into_owned(),
        })
        .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new text\r\n");
        let preview = undo_preview(
            &directory.path().join("state").to_string_lossy(),
            &manifest.id,
        )
        .unwrap();
        assert_eq!(preview.safe, vec![target.to_string_lossy()]);
        undo(UndoRequest {
            backup_root: directory
                .path()
                .join("state")
                .to_string_lossy()
                .into_owned(),
            operation_id: manifest.id,
            selected_paths: vec![],
            overwrite_conflicts: false,
        })
        .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"old text\r\n");
    }

    #[test]
    fn apply_refuses_stale_scan() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, b"old").unwrap();
        let scan = scan(request(&target)).unwrap();
        fs::write(&target, b"old but externally changed").unwrap();
        let manifest = apply(ApplyRequest {
            scan,
            selected_paths: vec![],
            selected_hunks: Default::default(),
            backup_retention: None,
            backup_root: directory
                .path()
                .join("state")
                .to_string_lossy()
                .into_owned(),
        })
        .unwrap();
        assert!(matches!(
            manifest.files[0].status,
            FileOperationStatus::Conflict
        ));
        assert_eq!(fs::read(&target).unwrap(), b"old but externally changed");
    }

    #[test]
    fn applies_only_selected_hunks() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, "old one\nunchanged\nold two\n").unwrap();
        let scan = scan(request(&target)).unwrap();
        assert_eq!(scan.files[0].hunks.len(), 2);
        let first_hunk = scan.files[0].hunks[0].id.clone();
        let path = target.to_string_lossy().into_owned();
        let manifest = apply(ApplyRequest {
            scan,
            selected_paths: vec![path.clone()],
            selected_hunks: [(path, vec![first_hunk])].into_iter().collect(),
            backup_root: directory
                .path()
                .join("state")
                .to_string_lossy()
                .into_owned(),
            backup_retention: None,
        })
        .unwrap();
        assert!(matches!(
            manifest.files[0].status,
            FileOperationStatus::Changed
        ));
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            "new one\nunchanged\nold two\n"
        );
    }

    #[test]
    fn cancelled_apply_stops_before_the_next_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, "old").unwrap();
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        let cancelled = AtomicBool::new(true);
        let result = apply_with_progress(
            ApplyRequest {
                scan: scan(request(&target)).unwrap(),
                selected_paths: vec![],
                selected_hunks: Default::default(),
                backup_root,
                backup_retention: None,
            },
            &cancelled,
            |_| {},
        );
        assert!(matches!(result, Err(Error::Cancelled)));
        assert_eq!(fs::read_to_string(target).unwrap(), "old");
        let operations =
            list_operations(&directory.path().join("state").to_string_lossy()).unwrap();
        assert_eq!(operations[0].status, OperationStatus::Cancelled);
        assert_eq!(operations[0].processed_files, 0);
    }

    #[test]
    fn partial_cancel_survives_reopen_and_undoes_only_changed_files() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.txt");
        let second = directory.path().join("second.txt");
        fs::write(&first, "old first").unwrap();
        fs::write(&second, "old second").unwrap();
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        let cancelled = AtomicBool::new(false);
        let result = apply_with_progress(
            ApplyRequest {
                scan: scan(request(directory.path())).unwrap(),
                selected_paths: vec![],
                selected_hunks: Default::default(),
                backup_root: backup_root.clone(),
                backup_retention: None,
            },
            &cancelled,
            |progress| {
                if matches!(progress.stage, ProgressStage::Applying) && progress.completed == 0 {
                    cancelled.store(true, Ordering::Relaxed);
                }
            },
        );
        assert!(matches!(result, Err(Error::Cancelled)));

        let reopened = list_operations(&backup_root).unwrap();
        let operation = &reopened[0];
        assert_eq!(operation.status, OperationStatus::Cancelled);
        assert_eq!(operation.total_files, 2);
        assert_eq!(operation.processed_files, 1);
        assert_eq!(
            operation
                .files
                .iter()
                .filter(|record| record.status == FileOperationStatus::Changed)
                .count(),
            1
        );
        let preview = undo_preview(&backup_root, &operation.id).unwrap();
        assert_eq!(preview.safe.len(), 1);
        undo(UndoRequest {
            backup_root,
            operation_id: operation.id.clone(),
            selected_paths: preview.safe,
            overwrite_conflicts: false,
        })
        .unwrap();
        assert_eq!(fs::read_to_string(first).unwrap(), "old first");
        assert_eq!(fs::read_to_string(second).unwrap(), "old second");
    }

    #[test]
    fn running_manifest_is_reconciled_after_restart_and_can_be_undone() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, "old").unwrap();
        let scan_result = scan(request(&target)).unwrap();
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        let id = Uuid::new_v4().to_string();
        let operation_directory = operation_dir(&backup_root, &id).unwrap();
        let objects = operation_directory.join("objects");
        fs::create_dir_all(&objects).unwrap();
        let prepared = prepare_one(
            &target,
            &scan_result.files[0],
            &scan_result.request.rules,
            None,
            None,
            &objects,
        )
        .unwrap();
        let manifest = OperationManifest {
            schema_version: 2,
            id: id.clone(),
            scan_id: scan_result.id,
            created_at: Utc::now().to_rfc3339(),
            status: OperationStatus::Running,
            finished_at: None,
            total_files: 1,
            processed_files: 0,
            files: vec![prepared.record.clone()],
        };
        write_manifest(&operation_directory, &manifest).unwrap();
        commit_prepared(&prepared).unwrap();

        let live_view = list_operations(&backup_root).unwrap();
        assert_eq!(live_view[0].status, OperationStatus::Running);
        assert_eq!(live_view[0].files[0].status, FileOperationStatus::Pending);
        let reopened = recover_interrupted_operations(&backup_root).unwrap();
        assert_eq!(reopened[0].status, OperationStatus::Interrupted);
        assert_eq!(reopened[0].processed_files, 1);
        assert_eq!(reopened[0].files[0].status, FileOperationStatus::Changed);
        let preview = undo_preview(&backup_root, &id).unwrap();
        assert_eq!(preview.safe, vec![target.to_string_lossy()]);
        undo(UndoRequest {
            backup_root,
            operation_id: id,
            selected_paths: vec![],
            overwrite_conflicts: false,
        })
        .unwrap();
        assert_eq!(fs::read_to_string(target).unwrap(), "old");
    }

    #[test]
    fn pending_file_not_yet_replaced_is_recovered_as_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, "old").unwrap();
        let scan_result = scan(request(&target)).unwrap();
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        let id = Uuid::new_v4().to_string();
        let operation_directory = operation_dir(&backup_root, &id).unwrap();
        let objects = operation_directory.join("objects");
        fs::create_dir_all(&objects).unwrap();
        let prepared = prepare_one(
            &target,
            &scan_result.files[0],
            &scan_result.request.rules,
            None,
            None,
            &objects,
        )
        .unwrap();
        write_manifest(
            &operation_directory,
            &OperationManifest {
                schema_version: 2,
                id,
                scan_id: scan_result.id,
                created_at: Utc::now().to_rfc3339(),
                status: OperationStatus::Running,
                finished_at: None,
                total_files: 1,
                processed_files: 0,
                files: vec![prepared.record],
            },
        )
        .unwrap();

        let reopened = recover_interrupted_operations(&backup_root).unwrap();
        assert_eq!(reopened[0].status, OperationStatus::Interrupted);
        assert_eq!(reopened[0].files[0].status, FileOperationStatus::Skipped);
        assert_eq!(fs::read_to_string(target).unwrap(), "old");
        assert!(
            undo_preview(&backup_root, &reopened[0].id)
                .unwrap()
                .safe
                .is_empty()
        );
    }

    #[test]
    fn backup_retention_removes_old_operations() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        for _ in 0..2 {
            fs::write(&target, "old").unwrap();
            apply(ApplyRequest {
                scan: scan(request(&target)).unwrap(),
                selected_paths: vec![],
                selected_hunks: Default::default(),
                backup_root: backup_root.clone(),
                backup_retention: Some(1),
            })
            .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(list_operations(&backup_root).unwrap().len(), 1);
    }

    #[test]
    fn undo_refuses_to_overwrite_external_changes() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        fs::write(&target, b"old").unwrap();
        let backup_root = directory
            .path()
            .join("state")
            .to_string_lossy()
            .into_owned();
        let manifest = apply(ApplyRequest {
            scan: scan(request(&target)).unwrap(),
            selected_paths: vec![],
            selected_hunks: Default::default(),
            backup_retention: None,
            backup_root: backup_root.clone(),
        })
        .unwrap();
        fs::write(&target, b"changed by another application").unwrap();
        let preview = undo_preview(&backup_root, &manifest.id).unwrap();
        assert_eq!(preview.conflicts, vec![target.to_string_lossy()]);
        let result = undo(UndoRequest {
            backup_root,
            operation_id: manifest.id,
            selected_paths: vec![],
            overwrite_conflicts: false,
        })
        .unwrap();
        assert!(matches!(
            result.files[0].status,
            FileOperationStatus::Conflict
        ));
        assert_eq!(
            fs::read(&target).unwrap(),
            b"changed by another application"
        );
    }

    #[test]
    fn operation_ids_must_be_canonical_uuids() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_string_lossy();

        assert!(undo_preview(&root, "..").is_err());
        assert!(undo_preview(&root, ".").is_err());
        assert!(undo_preview(&root, "not-a-uuid").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn apply_refuses_target_replaced_by_symlink_after_scan() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        let external = directory.path().join("external.txt");
        fs::write(&target, b"old").unwrap();
        fs::write(&external, b"old").unwrap();
        let scan = scan(request(&target)).unwrap();
        fs::remove_file(&target).unwrap();
        symlink(&external, &target).unwrap();

        let manifest = apply(ApplyRequest {
            scan,
            selected_paths: vec![],
            selected_hunks: Default::default(),
            backup_retention: None,
            backup_root: directory
                .path()
                .join("state")
                .to_string_lossy()
                .into_owned(),
        })
        .unwrap();

        assert!(matches!(
            manifest.files[0].status,
            FileOperationStatus::Conflict
        ));
        assert_eq!(fs::read(&external).unwrap(), b"old");
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[test]
    fn undo_refuses_symlink_even_when_conflict_overwrite_is_enabled() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sample.txt");
        let external = directory.path().join("external.txt");
        fs::write(&target, b"old").unwrap();
        fs::write(&external, b"external").unwrap();
        let scan = scan(request(&target)).unwrap();
        let backup_root = directory.path().join("state");
        let manifest = apply(ApplyRequest {
            scan,
            selected_paths: vec![],
            selected_hunks: Default::default(),
            backup_retention: None,
            backup_root: backup_root.to_string_lossy().into_owned(),
        })
        .unwrap();
        fs::remove_file(&target).unwrap();
        symlink(&external, &target).unwrap();

        let restored = undo(UndoRequest {
            backup_root: backup_root.to_string_lossy().into_owned(),
            operation_id: manifest.id,
            selected_paths: vec![],
            overwrite_conflicts: true,
        })
        .unwrap();

        assert!(matches!(
            restored.files[0].status,
            FileOperationStatus::Conflict
        ));
        assert_eq!(fs::read(&external).unwrap(), b"external");
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
