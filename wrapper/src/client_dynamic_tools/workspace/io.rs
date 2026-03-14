use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use super::LEGACY_WORKSPACE_MAX_SCAN_ENTRIES;
use super::MAX_FILE_BYTES;
use super::extract_limit;
use super::legacy_workspace_scan_budget_error;
use super::normalize_root_label;
use super::rel_display;
use super::resolve_workspace_path;
use super::workspace_root;

pub(crate) fn workspace_list_dir(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_list_dir expects an object argument".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let target = object
        .get("path")
        .and_then(Value::as_str)
        .map(|path| resolve_workspace_path(&root, path))
        .transpose()?
        .unwrap_or_else(|| root.clone());
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("failed to stat `{}`: {err}", target.display()))?;
    if !metadata.is_dir() {
        return Err(format!("`{}` is not a directory", target.display()));
    }
    let limit = extract_limit(object.get("limit"));
    let (entries, has_more_entries) = collect_sorted_dir_entries(&target, limit)?;

    let relative = rel_display(&root, &target);
    let mut rendered = vec![format!("Directory: {}", normalize_root_label(&relative))];
    if entries.is_empty() {
        rendered.push("(empty directory)".to_string());
        return Ok(rendered.join("\n"));
    }
    for path in entries {
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("failed to stat `{}`: {err}", path.display()))?;
        let kind = if metadata.is_dir() { "dir " } else { "file" };
        let size = if metadata.is_file() {
            format!("{} bytes", metadata.len())
        } else {
            "-".to_string()
        };
        rendered.push(format!("{kind}  {:<8} {}", size, rel_display(&root, &path)));
    }
    if has_more_entries {
        rendered.push("... more entries omitted".to_string());
    }
    Ok(rendered.join("\n"))
}

fn collect_sorted_dir_entries(
    target: &PathBuf,
    limit: usize,
) -> Result<(Vec<PathBuf>, bool), String> {
    let mut kept = Vec::with_capacity(limit);
    let mut has_more_entries = false;
    let mut scanned_entries = 0usize;
    let entries = fs::read_dir(target)
        .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?;
    for entry in entries {
        let entry = entry
            .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?;
        scanned_entries += 1;
        if scanned_entries > LEGACY_WORKSPACE_MAX_SCAN_ENTRIES {
            return Err(legacy_workspace_scan_budget_error());
        }
        insert_sorted_dir_entry(&mut kept, entry.path());
        if kept.len() > limit {
            kept.pop();
            has_more_entries = true;
        }
    }
    Ok((kept, has_more_entries))
}

fn insert_sorted_dir_entry(entries: &mut Vec<PathBuf>, candidate: PathBuf) {
    let insertion_index = entries
        .binary_search_by(|existing| compare_dir_entry_paths(existing, &candidate))
        .unwrap_or_else(|index| index);
    entries.insert(insertion_index, candidate);
}

fn compare_dir_entry_paths(left: &PathBuf, right: &PathBuf) -> std::cmp::Ordering {
    left.file_name()
        .cmp(&right.file_name())
        .then_with(|| left.cmp(right))
}

pub(crate) fn workspace_stat_path(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_stat_path expects an object argument".to_string())?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspace_stat_path requires `path`".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let target = resolve_workspace_path(&root, path)?;
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("failed to stat `{}`: {err}", target.display()))?;
    let relative = rel_display(&root, &target);
    let kind = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };
    let mut rendered = vec![
        format!("Path: {relative}"),
        format!("Type: {kind}"),
        format!("Size: {} bytes", metadata.len()),
    ];
    if let Ok(modified) = metadata.modified()
        && let Ok(value) = modified.duration_since(std::time::UNIX_EPOCH)
    {
        rendered.push(format!("Modified: {}", value.as_secs()));
    }
    Ok(rendered.join("\n"))
}

pub(crate) fn workspace_read_file(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_read_file expects an object argument".to_string())?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspace_read_file requires `path`".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let file_path = resolve_workspace_path(&root, path)?;
    let metadata = fs::metadata(&file_path)
        .map_err(|err| format!("failed to stat `{}`: {err}", file_path.display()))?;
    if !metadata.is_file() {
        return Err(format!("`{}` is not a regular file", file_path.display()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "`{}` is too large to read safely ({} bytes)",
            file_path.display(),
            metadata.len()
        ));
    }

    let contents = fs::read_to_string(&file_path).map_err(|err| {
        format!(
            "failed to read UTF-8 text from `{}`: {err}",
            file_path.display()
        )
    })?;
    let start_line = object
        .get("startLine")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1);
    let end_line = object
        .get("endLine")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    let relative = rel_display(&root, &file_path);

    let lines = contents.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(format!("File: {relative}\n(empty file)"));
    }
    let bounded_start = start_line.max(1).min(lines.len());
    let bounded_end = end_line
        .unwrap_or(lines.len())
        .max(bounded_start)
        .min(lines.len());
    let mut rendered = vec![format!("File: {relative}")];
    for (index, line) in lines
        .iter()
        .enumerate()
        .skip(bounded_start - 1)
        .take(bounded_end - bounded_start + 1)
    {
        rendered.push(format!("{:>4} | {}", index + 1, line));
    }
    Ok(rendered.join("\n"))
}
