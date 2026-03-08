use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

const MAX_FILE_BYTES: u64 = 256 * 1024;
const DEFAULT_LIMIT: usize = 20;
const MAX_RESULTS: usize = 100;
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".next", "dist", "build"];

pub(crate) fn dynamic_tool_specs() -> Value {
    Value::Array(vec![
        json!({
            "name": "workspace_read_file",
            "description": "Read a UTF-8 text file from the current workspace. Supports optional 1-based startLine and endLine filters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "startLine": {"type": "integer", "minimum": 1},
                    "endLine": {"type": "integer", "minimum": 1}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "workspace_find_files",
            "description": "Find workspace file paths whose relative path contains the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "workspace_search_text",
            "description": "Search UTF-8 text files in the current workspace for lines containing the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
    ])
}

pub(crate) fn execute_dynamic_tool_call(params: &Value, resolved_cwd: &str) -> Value {
    let tool = params
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or("dynamic tool");
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let result = match tool {
        "workspace_read_file" => workspace_read_file(arguments, resolved_cwd),
        "workspace_find_files" => workspace_find_files(arguments, resolved_cwd),
        "workspace_search_text" => workspace_search_text(arguments, resolved_cwd),
        _ => Err(format!("unsupported client dynamic tool `{tool}`")),
    };

    match result {
        Ok(text) => json!({
            "contentItems": [{"type": "inputText", "text": text}],
            "success": true
        }),
        Err(err) => json!({
            "contentItems": [{"type": "inputText", "text": err}],
            "success": false
        }),
    }
}

fn workspace_read_file(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

fn workspace_find_files(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

fn workspace_search_text(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
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

fn extract_limit(limit: Option<&Value>) -> usize {
    limit
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .map(|value| value.clamp(1, MAX_RESULTS))
        .unwrap_or(DEFAULT_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::dynamic_tool_specs;
    use super::execute_dynamic_tool_call;
    use serde_json::json;

    #[test]
    fn dynamic_tool_specs_include_workspace_tools() {
        let specs = dynamic_tool_specs();
        let names = specs
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "workspace_read_file",
                "workspace_find_files",
                "workspace_search_text"
            ]
        );
    }

    #[test]
    fn workspace_read_file_returns_line_numbered_content() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("hello.txt"), "alpha\nbeta\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_read_file",
                "arguments": {"path": "hello.txt", "startLine": 2}
            }),
            workspace.path().to_str().expect("utf8 path"),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("File: hello.txt"));
        assert!(text.contains("   2 | beta"));
    }

    #[test]
    fn workspace_search_text_returns_matching_lines() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("src.txt"), "alpha\nneedle here\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_search_text",
                "arguments": {"query": "needle"}
            }),
            workspace.path().to_str().expect("utf8 path"),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("Text matches for `needle`:"));
        assert!(text.contains("src.txt:2: needle here"));
    }

    #[test]
    fn workspace_find_files_returns_relative_paths() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
        std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_find_files",
                "arguments": {"query": "lib"}
            }),
            workspace.path().to_str().expect("utf8 path"),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("File matches for `lib`:"));
        assert!(text.contains("src/lib.rs"));
    }

    #[test]
    fn workspace_read_file_rejects_escape_outside_workspace() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::NamedTempFile::new().expect("tempfile");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_read_file",
                "arguments": {"path": outside.path()}
            }),
            workspace.path().to_str().expect("utf8 path"),
        );

        assert_eq!(result["success"], false);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("outside the current workspace"));
    }
}
