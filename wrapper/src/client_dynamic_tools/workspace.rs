use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

const MAX_FILE_BYTES: u64 = 256 * 1024;
const DEFAULT_LIMIT: usize = 20;
const MAX_RESULTS: usize = 100;
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".next", "dist", "build"];

pub(super) fn workspace_list_dir(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

pub(super) fn workspace_stat_path(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

pub(super) fn workspace_read_file(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

pub(super) fn workspace_find_files(
    arguments: &Value,
    resolved_cwd: &str,
) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_find_files expects an object argument".to_string())?;
    let query = object
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "workspace_find_files requires a non-empty `query`".to_string())?;
    let limit = extract_limit(object.get("limit"));
    let root = workspace_root(resolved_cwd)?;
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    walk_workspace(&root, &mut |path| {
        let relative = rel_display(&root, path);
        if relative.to_lowercase().contains(&query_lower) {
            matches.push(relative);
        }
        matches.len() < limit
    })?;

    if matches.is_empty() {
        return Ok(format!("No workspace files matched `{query}`."));
    }

    let mut rendered = vec![format!("File matches for `{query}`:")];
    rendered.extend(
        matches
            .iter()
            .enumerate()
            .map(|(index, path)| format!("{:>2}. {path}", index + 1)),
    );
    Ok(rendered.join("\n"))
}

pub(super) fn workspace_search_text(
    arguments: &Value,
    resolved_cwd: &str,
) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_search_text expects an object argument".to_string())?;
    let query = object
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "workspace_search_text requires a non-empty `query`".to_string())?;
    let limit = extract_limit(object.get("limit"));
    let root = workspace_root(resolved_cwd)?;
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    walk_workspace(&root, &mut |path| {
        if matches.len() >= limit {
            return false;
        }
        let Ok(metadata) = fs::metadata(path) else {
            return true;
        };
        if metadata.len() > MAX_FILE_BYTES {
            return true;
        }
        let Ok(contents) = fs::read_to_string(path) else {
            return true;
        };
        for (line_index, line) in contents.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                matches.push(format!(
                    "{}:{}: {}",
                    rel_display(&root, path),
                    line_index + 1,
                    line
                ));
                if matches.len() >= limit {
                    break;
                }
            }
        }
        true
    })?;

    if matches.is_empty() {
        return Ok(format!("No text matches for `{query}` in the workspace."));
    }

    let mut rendered = vec![format!("Text matches for `{query}`:")];
    rendered.extend(matches);
    Ok(rendered.join("\n"))
}

fn workspace_root(resolved_cwd: &str) -> Result<PathBuf, String> {
    let root = Path::new(resolved_cwd);
    fs::canonicalize(root)
        .map_err(|err| format!("failed to resolve workspace root `{resolved_cwd}`: {err}"))
}

fn resolve_workspace_path(root: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        root.join(raw_path)
    };
    let resolved = fs::canonicalize(&candidate)
        .map_err(|err| format!("failed to resolve `{}`: {err}", candidate.display()))?;
    if !resolved.starts_with(root) {
        return Err(format!(
            "`{}` is outside the current workspace",
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn walk_workspace(root: &Path, visit: &mut impl FnMut(&Path) -> bool) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read directory `{}`: {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read directory entry in `{}`: {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| format!("failed to stat `{}`: {err}", path.display()))?;
            if file_type.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if file_type.is_file() && !visit(&path) {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| SKIP_DIRS.iter().any(|skip| skip == &name))
}

fn rel_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn normalize_root_label(relative: &str) -> &str {
    if relative.is_empty() { "." } else { relative }
}

fn extract_limit(limit: Option<&Value>) -> usize {
    limit
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .map(|value| value.clamp(1, MAX_RESULTS))
        .unwrap_or(DEFAULT_LIMIT)
}
