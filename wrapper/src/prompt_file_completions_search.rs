use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

pub(crate) fn file_completions(token: &str, resolved_cwd: &str) -> Result<Vec<String>> {
    let token = token.trim();
    let (dir_part, name_prefix) = match token.rfind(['/', '\\']) {
        Some(idx) => (&token[..=idx], &token[idx + 1..]),
        None => ("", token),
    };
    let base_dir = if dir_part.is_empty() {
        PathBuf::from(resolved_cwd)
    } else {
        PathBuf::from(resolved_cwd).join(dir_part)
    };
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut matches = std::fs::read_dir(&base_dir)
        .with_context(|| format!("read directory {}", base_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = os_str_to_string(&name)?;
            if !name.starts_with(name_prefix) {
                return None;
            }
            let mut rendered = format!("{dir_part}{name}");
            if entry.path().is_dir() {
                rendered.push('/');
            }
            Some(rendered)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn os_str_to_string(value: &OsStr) -> Option<String> {
    value.to_str().map(ToOwned::to_owned)
}
