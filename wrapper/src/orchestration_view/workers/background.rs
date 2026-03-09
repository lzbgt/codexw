use crate::background_terminals::render_background_terminals;
use crate::state::AppState;

pub(super) fn render_server_background_terminals_only(state: &AppState) -> Option<Vec<String>> {
    let background = render_background_terminals(state);
    if background == "No background terminals running." {
        return None;
    }
    let lines = background
        .lines()
        .take_while(|line| *line != "Local background shell jobs:")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.is_empty() || lines[0] != "Server-observed background terminals:" {
        None
    } else {
        Some(lines)
    }
}
