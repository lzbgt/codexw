use std::fs;

use serde_json::Value;

use super::MAX_FILE_BYTES;
use super::extract_limit;
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
    let mut entries = fs::read_dir(&target)
        .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    let total_entries = entries.len();

    let relative = rel_display(&root, &target);
    let mut rendered = vec![format!("Directory: {}", normalize_root_label(&relative))];
    if entries.is_empty() {
        rendered.push("(empty directory)".to_string());
        return Ok(rendered.join("\n"));
    }
    for entry in entries.into_iter().take(limit) {
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|err| format!("failed to stat `{}`: {err}", path.display()))?;
        let kind = if metadata.is_dir() { "dir " } else { "file" };
        let size = if metadata.is_file() {
            format!("{} bytes", metadata.len())
        } else {
            "-".to_string()
        };
        rendered.push(format!("{kind}  {:<8} {}", size, rel_display(&root, &path)));
    }
    if rendered.len() == limit + 1 && total_entries > limit {
        rendered.push(format!(
            "... {} more entries omitted",
            total_entries - limit
        ));
    }
    Ok(rendered.join("\n"))
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
