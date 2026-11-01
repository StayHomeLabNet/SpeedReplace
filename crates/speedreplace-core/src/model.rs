use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid replacement rule: {0}")]
    InvalidRule(String),
    #[error("unsupported or ambiguous encoding: {0}")]
    Encoding(String),
    #[error("text cannot be represented in the original encoding: {0}")]
    Unmappable(String),
    #[error("scan is stale because the file changed: {0}")]
    Conflict(String),
    #[error("invalid operation manifest: {0}")]
    Manifest(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuleMode {
    Literal,
    Regex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementRule {
    pub id: String,
    pub enabled: bool,
    pub search: String,
    pub replacement: String,
    pub mode: RuleMode,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default = "default_true")]
    pub multiline: bool,
    #[serde(default)]
    pub description: String,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSpec {
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default = "default_excludes")]
    pub excludes: Vec<String>,
    #[serde(default = "default_max_size")]
    pub max_file_size: u64,
    #[serde(default = "default_true")]
    pub recursive: bool,
}

impl Default for FilterSpec {
    fn default() -> Self {
        Self {
            includes: vec![],
            excludes: default_excludes(),
            max_file_size: default_max_size(),
            recursive: true,
        }
    }
}

fn default_excludes() -> Vec<String> {
    [
        "**/.git/**",
        "**/.svn/**",
        "**/.hg/**",
        "**/node_modules/**",
        "**/target/**",
        "**/dist/**",
        "**/build/**",
        "**/.speedreplace/**",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn default_max_size() -> u64 {
    100 * 1024 * 1024
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub targets: Vec<String>,
    #[serde(default)]
    pub filters: FilterSpec,
    pub rules: Vec<ReplacementRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleValidation {
    pub rule_id: String,
    pub level: ValidationLevel,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationLevel {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingInfo {
    pub name: String,
    pub has_bom: bool,
    pub confidence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub path: String,
    pub source_hash: String,
    pub result_hash: String,
    pub encoding: Option<EncodingInfo>,
    pub line_endings: String,
    pub match_count: usize,
    pub diff: String,
    pub changed: bool,
    pub selected: bool,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub discovered: usize,
    pub scanned: usize,
    pub changed: usize,
    pub matches: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub id: String,
    pub request: ScanRequest,
    pub files: Vec<FilePreview>,
    pub summary: ScanSummary,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub scan: ScanResult,
    #[serde(default)]
    pub selected_paths: Vec<String>,
    pub backup_root: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileOperationStatus {
    Changed,
    Restored,
    Skipped,
    Conflict,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationFileRecord {
    pub path: String,
    pub source_hash: String,
    pub result_hash: String,
    pub backup_file: String,
    pub status: FileOperationStatus,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationManifest {
    pub schema_version: u32,
    pub id: String,
    pub scan_id: String,
    pub created_at: String,
    pub files: Vec<OperationFileRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoPreview {
    pub operation_id: String,
    pub safe: Vec<String>,
    pub conflicts: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoRequest {
    pub backup_root: String,
    pub operation_id: String,
    #[serde(default)]
    pub selected_paths: Vec<String>,
    #[serde(default)]
    pub overwrite_conflicts: bool,
}

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Error {
    Error::Io {
        path: path.into(),
        source,
    }
}
