use anyhow::Context;
use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

pub(crate) fn thread_id(state: &AppState) -> Result<&str> {
    state
        .thread_id
        .as_deref()
        .context("no active thread; wait for initialization or use :new")
}

pub(crate) fn summarize_text(text: &str) -> String {
    const LIMIT: usize = 120;
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= LIMIT {
        single_line
    } else {
        let truncated = single_line
            .chars()
            .take(LIMIT.saturating_sub(3))
            .collect::<String>();
        format!("{truncated}...")
    }
}

pub(crate) fn emit_status_line(
    _output: &mut Output,
    state: &mut AppState,
    line: String,
) -> Result<()> {
    if state.last_status_line.as_deref() == Some(line.as_str()) {
        return Ok(());
    }
    state.last_status_line = Some(line);
    Ok(())
}

pub(crate) fn canonicalize_or_keep(path: &str) -> String {
    std::fs::canonicalize(path)
        .ok()
        .and_then(|value| value.to_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| path.to_string())
}
