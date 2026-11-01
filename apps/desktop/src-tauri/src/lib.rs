use serde::Deserialize;
use speedreplace_core::{
    ApplyRequest, OperationManifest, ReplacementRule, RuleValidation, ScanRequest, ScanResult,
    UndoPreview, UndoRequest,
};
use tauri::Manager;

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
async fn scan_files(request: ScanRequest) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || speedreplace_core::scan(request))
        .await
        .map_err(user_error)?
        .map_err(user_error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyCommandRequest {
    scan: ScanResult,
    #[serde(default)]
    selected_paths: Vec<String>,
}

#[tauri::command]
async fn apply_changes(
    app: tauri::AppHandle,
    request: ApplyCommandRequest,
) -> Result<OperationManifest, String> {
    let request = ApplyRequest {
        scan: request.scan,
        selected_paths: request.selected_paths,
        backup_root: backup_root(&app)?,
    };
    tauri::async_runtime::spawn_blocking(move || speedreplace_core::apply(request))
        .await
        .map_err(user_error)?
        .map_err(user_error)
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            validate_rules,
            scan_files,
            apply_changes,
            list_operations,
            get_undo_preview,
            undo_operation
        ])
        .run(tauri::generate_context!())
        .expect("error while running SpeedReplace");
}
