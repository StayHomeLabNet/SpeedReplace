use clap::{Args, Parser, Subcommand};
use speedreplace_core::{
    ApplyRequest, FilterSpec, ReplacementRule, RuleMode, ScanRequest, UndoRequest,
    apply_with_progress, recover_interrupted_operations, scan_with_progress, undo,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
#[command(name = "speedreplace", version, about = "Safe batch text replacement")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan files and print a preview without writing.
    Scan(ReplaceArgs),
    /// Back up and replace matching text.
    Replace(ReplaceArgs),
    /// List backup operations.
    History(StateArgs),
    /// Restore an operation when destination files are unchanged.
    Undo {
        operation_id: String,
        #[command(flatten)]
        state: StateArgs,
        #[arg(long)]
        overwrite_conflicts: bool,
    },
}

#[derive(Args, Clone)]
struct StateArgs {
    #[arg(long, default_value = ".speedreplace")]
    backup_dir: PathBuf,
}

#[derive(Args)]
struct ReplaceArgs {
    #[arg(required = true)]
    targets: Vec<String>,
    #[arg(long, required_unless_present = "rules")]
    search: Option<String>,
    #[arg(long, default_value = "")]
    replacement: String,
    /// Load an array of replacement rules from JSON.
    #[arg(long)]
    rules: Option<PathBuf>,
    #[arg(long)]
    regex: bool,
    #[arg(long)]
    ignore_case: bool,
    #[arg(long)]
    whole_word: bool,
    #[arg(long)]
    include: Vec<String>,
    #[arg(long)]
    exclude: Vec<String>,
    #[arg(long, default_value_t = 100)]
    max_mib: u64,
    /// Force one supported encoding for all discovered files.
    #[arg(long)]
    encoding: Option<String>,
    #[arg(long, default_value = ".speedreplace")]
    backup_dir: PathBuf,
    #[arg(long, default_value_t = 20)]
    retain: usize,
    /// Emit machine-readable JSON instead of a human summary.
    #[arg(long)]
    json: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("speedreplace: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Scan(args) => {
            let result = scan_args(&args)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "{} changed files, {} matches",
                    result.summary.changed, result.summary.matches
                );
                for file in result.files.iter().filter(|file| file.changed) {
                    println!("\n{}\n{}", file.path, file.diff);
                }
            }
        }
        Command::Replace(args) => {
            let scan = scan_args(&args)?;
            let manifest = apply_with_progress(
                ApplyRequest {
                    scan,
                    selected_paths: vec![],
                    selected_hunks: BTreeMap::new(),
                    backup_root: args.backup_dir.to_string_lossy().into_owned(),
                    backup_retention: Some(args.retain),
                },
                &AtomicBool::new(false),
                |progress| eprintln!("Applying {}/{}", progress.completed, progress.total),
            )?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            } else {
                println!(
                    "Operation {}: {} files processed",
                    manifest.id,
                    manifest.files.len()
                );
            }
        }
        Command::History(state) => {
            let operations = recover_interrupted_operations(&state.backup_dir.to_string_lossy())?;
            for operation in operations {
                println!(
                    "{}\t{}\t{} files",
                    operation.id,
                    operation.created_at,
                    operation.files.len()
                );
            }
        }
        Command::Undo {
            operation_id,
            state,
            overwrite_conflicts,
        } => {
            let manifest = undo(UndoRequest {
                backup_root: state.backup_dir.to_string_lossy().into_owned(),
                operation_id,
                selected_paths: vec![],
                overwrite_conflicts,
            })?;
            println!("{} files inspected", manifest.files.len());
        }
    }
    Ok(())
}

fn scan_args(
    args: &ReplaceArgs,
) -> Result<speedreplace_core::ScanResult, Box<dyn std::error::Error>> {
    let rules = if let Some(path) = &args.rules {
        serde_json::from_slice(&fs::read(path)?)?
    } else {
        vec![ReplacementRule {
            id: uuid::Uuid::new_v4().to_string(),
            enabled: true,
            search: args.search.clone().unwrap_or_default(),
            replacement: args.replacement.clone(),
            mode: if args.regex {
                RuleMode::Regex
            } else {
                RuleMode::Literal
            },
            case_sensitive: !args.ignore_case,
            whole_word: args.whole_word,
            multiline: true,
            description: String::new(),
        }]
    };
    let default_filters = FilterSpec::default();
    let filters = FilterSpec {
        includes: args.include.clone(),
        excludes: if args.exclude.is_empty() {
            default_filters.excludes
        } else {
            default_filters
                .excludes
                .into_iter()
                .chain(args.exclude.iter().cloned())
                .collect()
        },
        max_file_size: args.max_mib * 1024 * 1024,
        recursive: true,
    };
    Ok(scan_with_progress(
        ScanRequest {
            targets: args.targets.clone(),
            filters,
            rules,
            encoding_overrides: BTreeMap::new(),
            default_encoding_override: args.encoding.clone(),
        },
        &AtomicBool::new(false),
        |progress| eprintln!("Scanning {}/{}", progress.completed, progress.total),
    )?)
}
