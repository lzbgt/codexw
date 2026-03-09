use serde_json::Value;

use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

#[derive(Debug, Clone, Default)]
pub(crate) struct BackgroundTerminalSummary {
    pub(crate) item_id: String,
    pub(crate) process_id: String,
    pub(crate) command_display: String,
    pub(crate) waiting: bool,
    pub(crate) recent_inputs: Vec<String>,
    pub(crate) recent_output: Vec<String>,
}

pub(crate) fn track_started_command_item(state: &mut AppState, item: &Value) {
    let Some(item_id) = get_string(item, &["id"]) else {
        return;
    };
    let Some(command) = get_string(item, &["command"]) else {
        return;
    };
    state
        .active_command_items
        .insert(item_id.to_string(), command.to_string());
}

pub(crate) fn track_terminal_interaction(state: &mut AppState, params: &Value) {
    let Some(item_id) = get_string(params, &["itemId"]) else {
        return;
    };
    let Some(process_id) = get_string(params, &["processId"]) else {
        return;
    };
    let stdin = get_string(params, &["stdin"]).unwrap_or("");
    let command_display = state
        .active_command_items
        .get(item_id)
        .map(|command| summarize_text(command))
        .unwrap_or_else(|| process_id.to_string());
    let seeded_output = recent_output_lines(state, item_id);

    let entry = state
        .orchestration
        .background_terminals
        .entry(process_id.to_string())
        .or_insert_with(|| BackgroundTerminalSummary {
            item_id: item_id.to_string(),
            process_id: process_id.to_string(),
            command_display: command_display.clone(),
            waiting: stdin.trim().is_empty(),
            recent_inputs: Vec::new(),
            recent_output: seeded_output,
        });
    entry.item_id = item_id.to_string();
    entry.command_display = command_display;
    entry.waiting = stdin.trim().is_empty();
    if !stdin.trim().is_empty() {
        let summarized = summarize_text(stdin.trim());
        if entry.recent_inputs.last() != Some(&summarized) {
            entry.recent_inputs.push(summarized);
            const MAX_RECENT_INPUTS: usize = 3;
            if entry.recent_inputs.len() > MAX_RECENT_INPUTS {
                let drop_count = entry.recent_inputs.len() - MAX_RECENT_INPUTS;
                entry.recent_inputs.drain(0..drop_count);
            }
        }
    }
}

pub(crate) fn track_command_output_delta(state: &mut AppState, params: &Value) {
    let Some(item_id) = get_string(params, &["itemId"]) else {
        return;
    };
    let Some(delta) = get_string(params, &["delta"]) else {
        return;
    };
    let recent_lines = summarize_recent_lines(delta);
    if recent_lines.is_empty() {
        return;
    }
    for process in state
        .orchestration
        .background_terminals
        .values_mut()
        .filter(|process| process.item_id == item_id)
    {
        for line in &recent_lines {
            process.recent_output.push(line.clone());
        }
        trim_recent_lines(&mut process.recent_output);
    }
}

pub(crate) fn clear_completed_command_item(state: &mut AppState, item: &Value) {
    let Some(item_id) = get_string(item, &["id"]) else {
        return;
    };
    state.active_command_items.remove(item_id);
    state
        .orchestration
        .background_terminals
        .retain(|_, process| process.item_id != item_id);
}

pub(crate) fn clear_all_background_terminals(state: &mut AppState) {
    state.orchestration.background_terminals.clear();
}

fn recent_output_lines(state: &AppState, item_id: &str) -> Vec<String> {
    state
        .command_output_buffers
        .get(item_id)
        .map(|buffer| summarize_recent_lines(buffer))
        .unwrap_or_default()
}

fn summarize_recent_lines(text: &str) -> Vec<String> {
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(summarize_text)
        .collect::<Vec<_>>();
    trim_recent_lines(&mut lines);
    lines
}

fn trim_recent_lines(lines: &mut Vec<String>) {
    const MAX_RECENT_LINES: usize = 3;
    if lines.len() > MAX_RECENT_LINES {
        let drop_count = lines.len() - MAX_RECENT_LINES;
        lines.drain(0..drop_count);
    }
}
