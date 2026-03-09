use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

const MAX_FILE_BYTES: u64 = 256 * 1024;
const DEFAULT_LIMIT: usize = 20;
const MAX_RESULTS: usize = 100;
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".next", "dist", "build"];

#[path = "workspace/io.rs"]
mod io;
#[path = "workspace/search.rs"]
mod search;

pub(crate) use io::workspace_list_dir;
pub(crate) use io::workspace_read_file;
pub(crate) use io::workspace_stat_path;
pub(crate) use search::workspace_find_files;
pub(crate) use search::workspace_search_text;

pub(super) fn workspace_root(resolved_cwd: &str) -> Result<PathBuf, String> {
    let root = Path::new(resolved_cwd);
    fs::canonicalize(root)
        .map_err(|err| format!("failed to resolve workspace root `{resolved_cwd}`: {err}"))
}

pub(super) fn resolve_workspace_path(root: &Path, raw_path: &str) -> Result<PathBuf, String> {
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

pub(super) fn walk_workspace(
    root: &Path,
    visit: &mut impl FnMut(&Path) -> bool,
) -> Result<(), String> {
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

pub(super) fn rel_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(super) fn normalize_root_label(relative: &str) -> &str {
    if relative.is_empty() { "." } else { relative }
}

pub(super) fn extract_limit(limit: Option<&Value>) -> usize {
    limit
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .map(|value| value.clamp(1, MAX_RESULTS))
        .unwrap_or(DEFAULT_LIMIT)
}
