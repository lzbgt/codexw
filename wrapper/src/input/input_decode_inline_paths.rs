use std::path::Path;
use std::path::PathBuf;

pub(crate) fn resolve_file_mention_path(token: &str, resolved_cwd: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let path = Path::new(token);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        PathBuf::from(resolved_cwd).join(path)
    };
    if !candidate.exists() {
        return None;
    }

    let rendered = if path.is_absolute() {
        token.to_string()
    } else {
        token.trim_start_matches("./").to_string()
    };
    if rendered.chars().any(char::is_whitespace) && !rendered.contains('"') {
        Some(format!("\"{rendered}\""))
    } else {
        Some(rendered)
    }
}
