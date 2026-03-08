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
        .background_terminals
        .retain(|_, process| process.item_id != item_id);
}

pub(crate) fn clear_all_background_terminals(state: &mut AppState) {
    state.background_terminals.clear();
}

pub(crate) fn server_background_terminal_count(state: &AppState) -> usize {
    state.background_terminals.len()
}

pub(crate) fn background_terminal_count(state: &AppState) -> usize {
    server_background_terminal_count(state) + state.background_shells.running_count()
}

pub(crate) fn render_background_terminals(state: &AppState) -> String {
    let mut processes = state
        .background_terminals
        .values()
        .cloned()
        .collect::<Vec<_>>();
    processes.sort_by(|left, right| {
        left.command_display
            .cmp(&right.command_display)
            .then_with(|| left.process_id.cmp(&right.process_id))
    });
    let mut lines = Vec::new();
    if !processes.is_empty() {
        lines.push("Server-observed background terminals:".to_string());
        for (index, process) in processes.iter().enumerate() {
            lines.push(format!(
                "{:>2}. {}  [{}]",
                index + 1,
                process.command_display,
                if process.waiting {
                    "waiting"
                } else {
                    "interactive"
                }
            ));
            lines.push(format!("    process  {}", process.process_id));
            if !process.recent_inputs.is_empty() {
                lines.push(format!(
                    "    recent   {}",
                    process.recent_inputs.join(" | ")
                ));
            }
            if !process.recent_output.is_empty() {
                lines.push(format!(
                    "    output   {}",
                    process.recent_output.join(" | ")
                ));
            }
        }
    }
    if let Some(local_jobs) = state.background_shells.render_for_ps() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(local_jobs);
    }
    if lines.is_empty() {
        return "No background terminals running.".to_string();
    }
    lines.push("Use /clean to stop all running background tasks.".to_string());
    lines.join("\n")
}

pub(crate) fn background_terminal_status_suffix(state: &AppState) -> Option<String> {
    let count = background_terminal_count(state);
    if count == 0 {
        None
    } else {
        Some(format!(
            "{count} background task{} running | /ps to view | /clean to close",
            if count == 1 { "" } else { "s" }
        ))
    }
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
