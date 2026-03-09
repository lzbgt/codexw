use std::fs;
use std::path::Path;

use serde_json::Value;

use super::MAX_FILE_BYTES;
use super::extract_limit;
use super::rel_display;
use super::walk_workspace;
use super::workspace_root;

pub(crate) fn workspace_find_files(
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

pub(crate) fn workspace_search_text(
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
        search_file_for_text(path, &root, &query_lower, &mut matches, limit);
        true
    })?;

    if matches.is_empty() {
        return Ok(format!("No text matches for `{query}` in the workspace."));
    }

    let mut rendered = vec![format!("Text matches for `{query}`:")];
    rendered.extend(matches);
    Ok(rendered.join("\n"))
}

fn search_file_for_text(
    path: &Path,
    root: &Path,
    query_lower: &str,
    matches: &mut Vec<String>,
    limit: usize,
) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() > MAX_FILE_BYTES {
        return;
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for (line_index, line) in contents.lines().enumerate() {
        if line.to_lowercase().contains(query_lower) {
            matches.push(format!(
                "{}:{}: {}",
                rel_display(root, path),
                line_index + 1,
                line
            ));
            if matches.len() >= limit {
                break;
            }
        }
    }
}
