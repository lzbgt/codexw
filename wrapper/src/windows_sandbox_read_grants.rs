use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;

pub(crate) const WINDOWS_SANDBOX_READ_ROOT_USAGE: &str =
    "Usage: /sandbox-add-read-dir <absolute-directory-path>";

pub(crate) fn parse_windows_sandbox_read_root_arg(raw_args: &str) -> Option<String> {
    let trimmed = raw_args.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(strip_matching_quotes(trimmed).trim().to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn grant_read_root_non_elevated(
    sandbox_mode: &str,
    cwd: &Path,
    codex_home: &Path,
    read_root: &Path,
) -> Result<PathBuf> {
    use codex_windows_sandbox::SandboxPolicy;
    use codex_windows_sandbox::run_setup_refresh_with_extra_read_roots;
    use std::collections::HashMap;

    if !read_root.is_absolute() {
        anyhow::bail!("path must be absolute: {}", read_root.display());
    }
    if !read_root.exists() {
        anyhow::bail!("path does not exist: {}", read_root.display());
    }
    if !read_root.is_dir() {
        anyhow::bail!("path must be a directory: {}", read_root.display());
    }

    let canonical_root = dunce::canonicalize(read_root)?;
    let policy = sandbox_policy_from_mode(sandbox_mode);
    let env_map: HashMap<String, String> = std::env::vars().collect();
    run_setup_refresh_with_extra_read_roots(
        &policy,
        cwd,
        cwd,
        &env_map,
        codex_home,
        vec![canonical_root.clone()],
    )?;
    Ok(canonical_root)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn grant_read_root_non_elevated(
    _sandbox_mode: &str,
    _cwd: &Path,
    _codex_home: &Path,
    _read_root: &Path,
) -> Result<PathBuf> {
    anyhow::bail!("Windows sandbox read-root refresh is only supported on Windows")
}

fn strip_matching_quotes(text: &str) -> &str {
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        let first = bytes[0];
        let last = bytes[text.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &text[1..text.len() - 1];
        }
    }
    text
}

#[cfg(target_os = "windows")]
fn sandbox_policy_from_mode(mode: &str) -> codex_windows_sandbox::SandboxPolicy {
    use codex_windows_sandbox::SandboxPolicy;

    match mode {
        "read-only" => SandboxPolicy::new_read_only_policy(),
        "workspace-write" => SandboxPolicy::new_workspace_write_policy(),
        _ => SandboxPolicy::DangerFullAccess,
    }
}

#[cfg(test)]
mod tests {
    use super::WINDOWS_SANDBOX_READ_ROOT_USAGE;
    use super::grant_read_root_non_elevated;
    use super::parse_windows_sandbox_read_root_arg;
    use std::path::Path;

    #[test]
    fn parse_windows_sandbox_read_root_arg_requires_input() {
        assert_eq!(parse_windows_sandbox_read_root_arg(""), None);
        assert_eq!(parse_windows_sandbox_read_root_arg("   "), None);
    }

    #[test]
    fn parse_windows_sandbox_read_root_arg_preserves_unquoted_paths() {
        assert_eq!(
            parse_windows_sandbox_read_root_arg("C:\\src\\project"),
            Some("C:\\src\\project".to_string())
        );
    }

    #[test]
    fn parse_windows_sandbox_read_root_arg_unwraps_matching_quotes() {
        assert_eq!(
            parse_windows_sandbox_read_root_arg("\"C:\\Program Files\\Code\""),
            Some("C:\\Program Files\\Code".to_string())
        );
        assert_eq!(
            parse_windows_sandbox_read_root_arg("'C:\\Program Files\\Code'"),
            Some("C:\\Program Files\\Code".to_string())
        );
    }

    #[test]
    fn usage_string_matches_upstream_shape() {
        assert_eq!(
            WINDOWS_SANDBOX_READ_ROOT_USAGE,
            "Usage: /sandbox-add-read-dir <absolute-directory-path>"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn grant_read_root_non_elevated_is_windows_only() {
        let err = grant_read_root_non_elevated(
            "workspace-write",
            Path::new("/tmp"),
            Path::new("/tmp"),
            Path::new("/tmp"),
        )
        .expect_err("non-Windows should not support read-root grants");
        assert!(err.to_string().contains("only supported on Windows"));
    }
}
