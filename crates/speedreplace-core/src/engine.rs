use crate::encoding::{decode, encode, line_endings};
use crate::{
    DiffHunk, FilePreview, OperationProgress, ProgressStage, ReplacementRule, Result, RuleMode,
    RuleValidation, ScanRequest, ScanResult, ScanSummary, ValidationLevel, io_error,
};
use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::{NoExpand, Regex, RegexBuilder};
use similar::{DiffTag, TextDiff};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;
use walkdir::WalkDir;

pub fn validate_rules(rules: &[ReplacementRule]) -> Vec<RuleValidation> {
    let mut result = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        if !rule.enabled {
            continue;
        }
        if rule.search.is_empty() {
            result.push(validation(
                rule,
                ValidationLevel::Error,
                "Search text cannot be empty",
            ));
            continue;
        }
        if rule.mode == RuleMode::Regex && compile_rule(rule).is_err() {
            result.push(validation(
                rule,
                ValidationLevel::Error,
                "Invalid regular expression",
            ));
        }
        if rule.search == rule.replacement {
            result.push(validation(
                rule,
                ValidationLevel::Warning,
                "This rule does not change text",
            ));
        }
        for later in rules
            .iter()
            .skip(index + 1)
            .filter(|candidate| candidate.enabled)
        {
            if later.mode == RuleMode::Literal
                && !later.search.is_empty()
                && rule.replacement.contains(&later.search)
            {
                result.push(validation(
                    rule,
                    ValidationLevel::Warning,
                    &format!("Replacement may feed into later rule {}", later.id),
                ));
            }
        }
    }
    result
}

fn validation(rule: &ReplacementRule, level: ValidationLevel, message: &str) -> RuleValidation {
    RuleValidation {
        rule_id: rule.id.clone(),
        level,
        message: message.to_owned(),
    }
}

fn compile_rule(rule: &ReplacementRule) -> std::result::Result<Regex, regex::Error> {
    let mut expression = match rule.mode {
        RuleMode::Literal => regex::escape(&rule.search),
        RuleMode::Regex => rule.search.clone(),
    };
    if rule.whole_word && rule.mode == RuleMode::Literal {
        expression = format!(r"\b(?:{expression})\b");
    }
    RegexBuilder::new(&expression)
        .case_insensitive(!rule.case_sensitive)
        .multi_line(rule.multiline)
        .dot_matches_new_line(rule.multiline)
        .build()
}

pub(crate) fn transform(text: &str, rules: &[ReplacementRule]) -> Result<(String, usize)> {
    let mut current = text.to_owned();
    let mut total = 0;
    for rule in rules.iter().filter(|rule| rule.enabled) {
        let regex = compile_rule(rule)
            .map_err(|error| crate::Error::InvalidRule(format!("{}: {error}", rule.id)))?;
        let count = regex.find_iter(&current).count();
        if count == 0 {
            continue;
        }
        current = match rule.mode {
            RuleMode::Literal => regex
                .replace_all(&current, NoExpand(&rule.replacement))
                .into_owned(),
            RuleMode::Regex => regex
                .replace_all(&current, rule.replacement.as_str())
                .into_owned(),
        };
        total += count;
    }
    Ok((current, total))
}

pub fn scan(request: ScanRequest) -> Result<ScanResult> {
    scan_with_progress(request, &AtomicBool::new(false), |_| {})
}

pub fn scan_with_progress(
    request: ScanRequest,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(OperationProgress),
) -> Result<ScanResult> {
    let errors = validate_rules(&request.rules)
        .into_iter()
        .filter(|item| matches!(item.level, ValidationLevel::Error))
        .map(|item| item.message)
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(crate::Error::InvalidRule(errors.join("; ")));
    }

    let include = build_globs(&request.filters.includes)?;
    let exclude = build_globs(&request.filters.excludes)?;
    let files = discover(
        &request,
        include.as_ref(),
        exclude.as_ref(),
        cancelled,
        &mut progress,
    )?;
    let mut summary = ScanSummary {
        discovered: files.len(),
        ..Default::default()
    };
    let mut previews = Vec::with_capacity(files.len());

    let total = files.len();
    for (index, path) in files.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err(crate::Error::Cancelled);
        }
        progress(OperationProgress {
            stage: ProgressStage::Scanning,
            completed: index,
            total,
            current_path: Some(path.to_string_lossy().into_owned()),
        });
        let encoding_override = request
            .encoding_overrides
            .get(path.to_string_lossy().as_ref())
            .or(request.default_encoding_override.as_ref())
            .map(String::as_str);
        match preview_file(
            &path,
            &request.rules,
            request.filters.max_file_size,
            encoding_override,
        ) {
            Ok(preview) => {
                summary.scanned += 1;
                if preview.changed {
                    summary.changed += 1;
                    summary.matches += preview.match_count;
                }
                previews.push(preview);
            }
            Err(error) => {
                summary.failed += 1;
                previews.push(FilePreview {
                    path: path.to_string_lossy().into_owned(),
                    source_hash: String::new(),
                    result_hash: String::new(),
                    encoding: None,
                    line_endings: "unknown".to_owned(),
                    match_count: 0,
                    diff: String::new(),
                    hunks: vec![],
                    changed: false,
                    selected: false,
                    warnings: vec![],
                    error: Some(error.to_string()),
                });
            }
        }
    }
    progress(OperationProgress {
        stage: ProgressStage::Scanning,
        completed: total,
        total,
        current_path: None,
    });

    previews.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ScanResult {
        id: Uuid::new_v4().to_string(),
        request,
        files: previews,
        summary,
        created_at: Utc::now().to_rfc3339(),
    })
}

fn build_globs(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).map_err(|error| {
            crate::Error::InvalidRule(format!("invalid glob {pattern}: {error}"))
        })?);
    }
    builder
        .build()
        .map(Some)
        .map_err(|error| crate::Error::InvalidRule(format!("invalid filters: {error}")))
}

fn discover(
    request: &ScanRequest,
    include: Option<&GlobSet>,
    exclude: Option<&GlobSet>,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(OperationProgress),
) -> Result<Vec<PathBuf>> {
    let mut found = BTreeSet::new();
    for target in &request.targets {
        if cancelled.load(Ordering::Relaxed) {
            return Err(crate::Error::Cancelled);
        }
        let target = PathBuf::from(target);
        let target_type = fs::symlink_metadata(&target)
            .ok()
            .map(|metadata| metadata.file_type());
        if target_type.is_some_and(|file_type| file_type.is_symlink()) {
            continue;
        }
        if target_type.is_some_and(|file_type| file_type.is_file()) {
            if accepted(&target, include, exclude) {
                found.insert(target);
            }
            continue;
        }
        if !target_type.is_some_and(|file_type| file_type.is_dir()) {
            continue;
        }
        let mut walker = WalkDir::new(&target).follow_links(false);
        if !request.filters.recursive {
            walker = walker.max_depth(1);
        }
        for entry in walker.into_iter().filter_map(std::result::Result::ok) {
            if cancelled.load(Ordering::Relaxed) {
                return Err(crate::Error::Cancelled);
            }
            if entry.file_type().is_file() && accepted(entry.path(), include, exclude) {
                found.insert(entry.into_path());
                progress(OperationProgress {
                    stage: ProgressStage::Scanning,
                    completed: found.len(),
                    total: 0,
                    current_path: None,
                });
            }
        }
    }
    Ok(found.into_iter().collect())
}

fn accepted(path: &Path, include: Option<&GlobSet>, exclude: Option<&GlobSet>) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if exclude.is_some_and(|patterns| patterns.is_match(&normalized)) {
        return false;
    }
    include.is_none_or(|patterns| patterns.is_match(&normalized))
}

fn preview_file(
    path: &Path,
    rules: &[ReplacementRule],
    max_size: u64,
    encoding_override: Option<&str>,
) -> Result<FilePreview> {
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.len() > max_size {
        return Err(crate::Error::Encoding(format!(
            "file exceeds {} byte limit",
            max_size
        )));
    }
    let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
    if looks_binary(&bytes) {
        return Err(crate::Error::Encoding("binary file skipped".to_owned()));
    }
    let source_hash = blake3::hash(&bytes).to_hex().to_string();
    let decoded = decode(&bytes, encoding_override)?;
    let endings = line_endings(&decoded.text);
    let (after, matches) = transform(&decoded.text, rules)?;
    let output = encode(&after, &decoded)?;
    let changed = output != bytes;
    let text_diff = TextDiff::from_lines(&decoded.text, &after);
    let diff = if changed {
        text_diff
            .unified_diff()
            .header("before", "after")
            .context_radius(3)
            .to_string()
    } else {
        String::new()
    };
    let hunks = if changed {
        build_hunks(&text_diff)
    } else {
        vec![]
    };
    Ok(FilePreview {
        path: path.to_string_lossy().into_owned(),
        source_hash,
        result_hash: blake3::hash(&output).to_hex().to_string(),
        encoding: Some(decoded.info),
        line_endings: endings.clone(),
        match_count: matches,
        diff,
        hunks,
        changed,
        selected: changed,
        warnings: if endings == "mixed" {
            vec!["Mixed line endings are preserved where untouched".to_owned()]
        } else {
            vec![]
        },
        error: None,
    })
}

pub(crate) fn build_hunks_for_text(source: &str, result: &str) -> Vec<DiffHunk> {
    let diff = TextDiff::from_lines(source, result);
    build_hunks(&diff)
}

fn build_hunks(diff: &TextDiff<'_, '_, '_, str>) -> Vec<DiffHunk> {
    diff.ops()
        .iter()
        .filter(|op| op.tag() != DiffTag::Equal)
        .enumerate()
        .map(|(index, op)| {
            let old = op.old_range();
            let new = op.new_range();
            let replacement = diff.new_slices()[new.clone()].concat();
            let mut display = format!(
                "@@ -{},{} +{},{} @@\n",
                old.start + 1,
                old.len(),
                new.start + 1,
                new.len()
            );
            for line in &diff.old_slices()[old.clone()] {
                display.push('-');
                display.push_str(line);
                if !line.ends_with('\n') {
                    display.push('\n');
                }
            }
            for line in &diff.new_slices()[new] {
                display.push('+');
                display.push_str(line);
                if !line.ends_with('\n') {
                    display.push('\n');
                }
            }
            DiffHunk {
                id: format!("hunk-{index}"),
                old_start: old.start,
                old_len: old.len(),
                replacement,
                diff: display,
            }
        })
        .collect()
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return false;
    }
    bytes.iter().take(8192).any(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(search: &str, replacement: &str) -> ReplacementRule {
        ReplacementRule {
            id: "one".into(),
            enabled: true,
            search: search.into(),
            replacement: replacement.into(),
            mode: RuleMode::Literal,
            case_sensitive: true,
            whole_word: false,
            multiline: true,
            description: String::new(),
        }
    }

    #[test]
    fn literal_replacement_does_not_expand_dollar_signs() {
        let (output, count) = transform("hello hello", &[rule("hello", "$1")]).unwrap();
        assert_eq!(output, "$1 $1");
        assert_eq!(count, 2);
    }

    #[test]
    fn ordered_rules_feed_forward() {
        let rules = [rule("a", "b"), rule("b", "c")];
        let (output, count) = transform("a", &rules).unwrap();
        assert_eq!(output, "c");
        assert_eq!(count, 2);
    }

    #[test]
    fn regex_supports_named_captures_and_multiline() {
        let regex_rule = ReplacementRule {
            id: "regex".into(),
            enabled: true,
            search: r"(?m)^(?<key>\w+)=(?<value>[^\n]+)$".into(),
            replacement: "${key}: ${value}".into(),
            mode: RuleMode::Regex,
            case_sensitive: true,
            whole_word: false,
            multiline: true,
            description: String::new(),
        };
        let (output, count) = transform("one=1\ntwo=2", &[regex_rule]).unwrap();
        assert_eq!(output, "one: 1\ntwo: 2");
        assert_eq!(count, 2);
    }

    #[test]
    fn scan_respects_include_and_exclude_patterns() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("keep.txt"), "old").unwrap();
        fs::write(directory.path().join("skip.md"), "old").unwrap();
        let ignored = directory.path().join("node_modules");
        fs::create_dir(&ignored).unwrap();
        fs::write(ignored.join("hidden.txt"), "old").unwrap();
        let result = scan(ScanRequest {
            targets: vec![directory.path().to_string_lossy().into_owned()],
            filters: crate::FilterSpec {
                includes: vec!["**/*.txt".into()],
                ..Default::default()
            },
            rules: vec![rule("old", "new")],
            encoding_overrides: Default::default(),
            default_encoding_override: None,
        })
        .unwrap();
        assert_eq!(result.summary.discovered, 1);
        assert_eq!(result.summary.changed, 1);
        assert!(result.files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn binary_files_are_never_previewed_as_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("data.bin");
        fs::write(&path, b"old\0value").unwrap();
        let result = scan(ScanRequest {
            targets: vec![path.to_string_lossy().into_owned()],
            filters: crate::FilterSpec::default(),
            rules: vec![rule("old", "new")],
            encoding_overrides: Default::default(),
            default_encoding_override: None,
        })
        .unwrap();
        assert!(!result.files[0].changed);
        assert!(result.files[0].error.as_deref().unwrap().contains("binary"));
    }

    #[test]
    fn cancelled_scan_stops_before_reading_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sample.txt");
        fs::write(&path, "old").unwrap();
        let cancelled = AtomicBool::new(true);
        let result = scan_with_progress(
            ScanRequest {
                targets: vec![path.to_string_lossy().into_owned()],
                filters: crate::FilterSpec::default(),
                rules: vec![rule("old", "new")],
                encoding_overrides: Default::default(),
                default_encoding_override: None,
            },
            &cancelled,
            |_| {},
        );
        assert!(matches!(result, Err(crate::Error::Cancelled)));
    }

    #[cfg(unix)]
    #[test]
    fn direct_symlink_targets_are_skipped() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("original.txt");
        let link = directory.path().join("link.txt");
        fs::write(&original, "old").unwrap();
        symlink(&original, &link).unwrap();

        let result = scan(ScanRequest {
            targets: vec![link.to_string_lossy().into_owned()],
            filters: crate::FilterSpec::default(),
            rules: vec![rule("old", "new")],
            encoding_overrides: Default::default(),
            default_encoding_override: None,
        })
        .unwrap();

        assert_eq!(result.summary.discovered, 0);
        assert!(result.files.is_empty());
        assert_eq!(fs::read_to_string(original).unwrap(), "old");
    }
}
