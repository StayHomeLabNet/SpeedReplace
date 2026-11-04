use serde::{Deserialize, Serialize};
use speedreplace_core::{
    ApplyRequest, OperationManifest, OperationProgress, ReplacementRule, RuleValidation,
    ScanRequest, ScanResult, UndoPreview, UndoRequest,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct OperationControls(Mutex<HashMap<String, Arc<AtomicBool>>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    job_id: String,
    progress: OperationProgress,
}

fn user_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn backup_root(app: &tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("backups").to_string_lossy().into_owned())
        .map_err(user_error)
}

#[tauri::command]
fn validate_rules(rules: Vec<ReplacementRule>) -> Vec<RuleValidation> {
    speedreplace_core::validate_rules(&rules)
}

#[tauri::command]
async fn scan_files(
    app: tauri::AppHandle,
    controls: State<'_, OperationControls>,
    job_id: String,
    request: ScanRequest,
) -> Result<ScanResult, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    controls
        .0
        .lock()
        .map_err(user_error)?
        .insert(job_id.clone(), cancelled.clone());
    let app_for_progress = app.clone();
    let event_job_id = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        speedreplace_core::scan_with_progress(request, &cancelled, |progress| {
            let _ = app_for_progress.emit(
                "operation-progress",
                ProgressEvent {
                    job_id: event_job_id.clone(),
                    progress,
                },
            );
        })
    })
    .await
    .map_err(user_error)?;
    controls.0.lock().map_err(user_error)?.remove(&job_id);
    result.map_err(user_error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyCommandRequest {
    scan: ScanResult,
    #[serde(default)]
    selected_paths: Vec<String>,
    #[serde(default)]
    selected_hunks: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    backup_retention: Option<usize>,
}

#[tauri::command]
async fn apply_changes(
    app: tauri::AppHandle,
    controls: State<'_, OperationControls>,
    job_id: String,
    request: ApplyCommandRequest,
) -> Result<OperationManifest, String> {
    let request = ApplyRequest {
        scan: request.scan,
        selected_paths: request.selected_paths,
        selected_hunks: request.selected_hunks,
        backup_root: backup_root(&app)?,
        backup_retention: request.backup_retention,
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    controls
        .0
        .lock()
        .map_err(user_error)?
        .insert(job_id.clone(), cancelled.clone());
    let app_for_progress = app.clone();
    let event_job_id = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        speedreplace_core::apply_with_progress(request, &cancelled, |progress| {
            let _ = app_for_progress.emit(
                "operation-progress",
                ProgressEvent {
                    job_id: event_job_id.clone(),
                    progress,
                },
            );
        })
    })
    .await
    .map_err(user_error)?;
    controls.0.lock().map_err(user_error)?.remove(&job_id);
    result.map_err(user_error)
}

#[tauri::command]
fn cancel_operation(controls: State<'_, OperationControls>, job_id: String) -> bool {
    controls
        .0
        .lock()
        .ok()
        .and_then(|items| items.get(&job_id).cloned())
        .is_some_and(|cancelled| {
            cancelled.store(true, Ordering::Relaxed);
            true
        })
}

#[tauri::command]
async fn list_operations(app: tauri::AppHandle) -> Result<Vec<OperationManifest>, String> {
    let root = backup_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || speedreplace_core::list_operations(&root))
        .await
        .map_err(user_error)?
        .map_err(user_error)
}

#[tauri::command]
async fn get_undo_preview(
    app: tauri::AppHandle,
    operation_id: String,
) -> Result<UndoPreview, String> {
    let root = backup_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        speedreplace_core::undo_preview(&root, &operation_id)
    })
    .await
    .map_err(user_error)?
    .map_err(user_error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UndoCommandRequest {
    operation_id: String,
    #[serde(default)]
    selected_paths: Vec<String>,
    #[serde(default)]
    overwrite_conflicts: bool,
}

#[tauri::command]
async fn undo_operation(
    app: tauri::AppHandle,
    request: UndoCommandRequest,
) -> Result<OperationManifest, String> {
    let request = UndoRequest {
        backup_root: backup_root(&app)?,
        operation_id: request.operation_id,
        selected_paths: request.selected_paths,
        overwrite_conflicts: request.overwrite_conflicts,
    };
    tauri::async_runtime::spawn_blocking(move || speedreplace_core::undo(request))
        .await
        .map_err(user_error)?
        .map_err(user_error)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(OperationControls::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            validate_rules,
            scan_files,
            apply_changes,
            list_operations,
            get_undo_preview,
            undo_operation,
            cancel_operation
        ])
        .run(tauri::generate_context!())
        .expect("error while running SpeedReplace");
}
