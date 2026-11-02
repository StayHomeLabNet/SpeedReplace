use crate::encoding::{decode, encode};
use crate::engine::transform;
use crate::{
    ApplyRequest, Error, FileOperationStatus, OperationFileRecord, OperationManifest, Result,
    UndoPreview, UndoRequest, io_error,
};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;

pub fn apply(request: ApplyRequest) -> Result<OperationManifest> {
    let operation_id = Uuid::new_v4().to_string();
    let operation_dir = operation_dir(&request.backup_root, &operation_id)?;
    let objects_dir = operation_dir.join("objects");
    fs::create_dir_all(&objects_dir).map_err(|error| io_error(&objects_dir, error))?;
    let selected: HashSet<&str> = request.selected_paths.iter().map(String::as_str).collect();
    let apply_all = selected.is_empty();
    let mut manifest = OperationManifest {
        schema_version: 1,
        id: operation_id,
        scan_id: request.scan.id.clone(),
        created_at: Utc::now().to_rfc3339(),
        files: vec![],
    };

    for preview in request.scan.files.iter().filter(|file| {
        file.changed && file.error.is_none() && (apply_all || selected.contains(file.path.as_str()))
    }) {
        let path = PathBuf::from(&preview.path);
        let record = match apply_one(&path, preview, &request.scan.request.rules, &objects_dir) {
            Ok(record) => record,
            Err(error) => OperationFileRecord {
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
            },
        };
        manifest.files.push(record);
        write_manifest(&operation_dir, &manifest)?;
    }
    write_manifest(&operation_dir, &manifest)?;
    Ok(manifest)
}

fn apply_one(
    path: &Path,
    preview: &crate::FilePreview,
    rules: &[crate::ReplacementRule],
    objects_dir: &Path,
) -> Result<OperationFileRecord> {
    reject_symlink(path)?;
    let original = fs::read(path).map_err(|error| io_error(path, error))?;
    let current_hash = blake3::hash(&original).to_hex().to_string();
    if current_hash != preview.source_hash {
        return Err(Error::Conflict(preview.path.clone()));
    }
    let decoded = decode(&original)?;
    let (after, _) = transform(&decoded.text, rules)?;
    let output = encode(&after, &decoded)?;
    let result_hash = blake3::hash(&output).to_hex().to_string();
    if result_hash != preview.result_hash {
        return Err(Error::Conflict(format!(
            "preview result no longer matches for {}",
            preview.path
        )));
    }

    let backup = objects_dir.join(&preview.source_hash);
    if !backup.exists() {
        fs::write(&backup, &original).map_err(|error| io_error(&backup, error))?;
    }
    let verified = fs::read(&backup).map_err(|error| io_error(&backup, error))?;
    if blake3::hash(&verified).to_hex().as_str() != preview.source_hash {
        return Err(Error::Manifest(format!(
            "backup verification failed for {}",
            preview.path
        )));
    }

    replace_file(path, &output)?;
    let written = fs::read(path).map_err(|error| io_error(path, error))?;
    if blake3::hash(&written).to_hex().as_str() != result_hash {
        return Err(Error::Conflict(format!(
            "post-write verification failed for {}",
            preview.path
        )));
    }
    Ok(OperationFileRecord {
        path: preview.path.clone(),
        source_hash: preview.source_hash.clone(),
        result_hash,
        backup_file: backup.to_string_lossy().into_owned(),
        status: FileOperationStatus::Changed,
        message: None,
    })
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
    let root = PathBuf::from(backup_root).join("operations");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut manifests = vec![];
    for entry in fs::read_dir(&root).map_err(|error| io_error(&root, error))? {
        let entry = entry.map_err(|error| io_error(&root, error))?;
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.is_file()
            && let Ok(manifest) = read_manifest(&manifest_path)
        {
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

fn read_manifest(path: &Path) -> Result<OperationManifest> {
    let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
    serde_json::from_slice(&bytes).map_err(|error| Error::Manifest(error.to_string()))
}

fn load_operation(backup_root: &str, operation_id: &str) -> Result<OperationManifest> {
    let directory = operation_dir(backup_root, operation_id)?;
    let manifest = read_manifest(&directory.join("manifest.json"))?;
    if manifest.schema_version != 1 || manifest.id != operation_id {
        return Err(Error::Manifest(
            "operation manifest identity or schema does not match".into(),
        ));
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
