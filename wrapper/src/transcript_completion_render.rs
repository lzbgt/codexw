use serde_json::Value;

use crate::state::get_string;
use crate::transcript_item_summary::summarize_file_change_paths;

const MAX_RENDERED_RESULT_LINES: usize = 80;
const HEAD_RESULT_LINES: usize = 20;
const TAIL_RESULT_LINES: usize = 20;

pub(crate) fn render_command_completion(
    command: &str,
    status: &str,
    exit_code: &str,
    output: Option<&str>,
    verbose: bool,
) -> String {
    let mut rendered = format!("$ {command}");
    if !is_successful_command_completion(status, exit_code) {
        rendered.push_str(&format!("\nstatus  {status}\nexit    {exit_code}"));
        if let Some(hint) = detect_shell_failure_hint(command, output.unwrap_or("")) {
            rendered.push_str(&format!("\nhint    {hint}"));
        }
    }
    if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(&abbreviate_long_result_text(trimmed, verbose));
        }
    }
    rendered
}

fn is_successful_command_completion(status: &str, exit_code: &str) -> bool {
    status == "completed" && exit_code == "0"
}

fn detect_shell_failure_hint(command: &str, output: &str) -> Option<&'static str> {
    if command.contains("status=$?") && output.contains("read-only variable: status") {
        return Some("zsh rejects `status=$?`; use `rc=$?` instead");
    }
    None
}

pub(crate) fn render_local_command_completion(
    command: &str,
    exit_code: &str,
    stdout: &str,
    stderr: &str,
    verbose: bool,
) -> String {
    let mut rendered = format!("$ {command}");
    let stdout = stdout.trim_end();
    let stderr = stderr.trim_end();
    let show_exit = exit_code != "0";
    let show_stdout = !stdout.trim().is_empty();
    let show_stderr = !stderr.trim().is_empty();
    let show_stream_labels = show_stdout && show_stderr;

    if show_exit {
        rendered.push_str(&format!("\nexit    {exit_code}"));
        if let Some(hint) = detect_shell_failure_hint(command, stderr) {
            rendered.push_str(&format!("\nhint    {hint}"));
        }
    }
    if show_stdout {
        rendered.push_str("\n\n");
        if show_stream_labels {
            rendered.push_str("[stdout]\n");
        }
        rendered.push_str(&abbreviate_long_result_text(stdout, verbose));
    }
    if show_stderr {
        rendered.push_str("\n\n");
        if show_stream_labels {
            rendered.push_str("[stderr]\n");
        }
        rendered.push_str(&abbreviate_long_result_text(stderr, verbose));
    }
    rendered
}

pub(crate) fn render_file_change_completion(
    item: &Value,
    status: &str,
    output: Option<&str>,
    _verbose: bool,
) -> String {
    let structured = render_file_changes(item);
    let has_structured = !structured.is_empty();
    let mut rendered = if has_structured {
        String::new()
    } else {
        summarize_file_change_paths(item)
    };
    if status != "completed" {
        if !rendered.is_empty() {
            rendered = format!("status  {status}\n{rendered}");
        } else {
            rendered = format!("status  {status}");
        }
    }
    if has_structured {
        if !rendered.is_empty() {
            rendered.push_str("\n\n");
        }
        rendered.push_str(&structured);
    } else if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            if !rendered.is_empty() {
                rendered.push_str("\n\n");
            }
            rendered.push_str(trimmed);
        }
    }
    rendered
}

pub(crate) fn abbreviate_long_result_text(text: &str, verbose: bool) -> String {
    if verbose {
        return text.to_string();
    }
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() <= MAX_RENDERED_RESULT_LINES {
        return text.to_string();
    }

    let mut abbreviated = Vec::with_capacity(HEAD_RESULT_LINES + TAIL_RESULT_LINES + 1);
    abbreviated.extend(lines.iter().take(HEAD_RESULT_LINES).copied());
    abbreviated.push("...");
    abbreviated.extend(
        lines
            .iter()
            .skip(lines.len().saturating_sub(TAIL_RESULT_LINES))
            .copied(),
    );
    abbreviated.join("\n")
}

pub(crate) fn render_pending_attachments(
    local_images: &[String],
    remote_images: &[String],
) -> String {
    let mut lines = Vec::new();
    for path in local_images {
        lines.push(format!("local-image  {path}"));
    }
    for url in remote_images {
        lines.push(format!("remote-image {url}"));
    }
    lines.join("\n")
}

pub(crate) fn render_file_changes(item: &Value) -> String {
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return String::new();
    };
    let mut rendered = String::new();
    for (idx, change) in changes.iter().enumerate() {
        if idx > 0 {
            rendered.push_str("\n\n");
        }
        let path = get_string(change, &["path"]).unwrap_or("?");
        let kind = get_string(change, &["kind"]).unwrap_or("?");
        rendered.push_str(&format!("{kind} {path}"));
        if let Some(diff) = get_string(change, &["diff"])
            && !diff.is_empty()
        {
            rendered.push_str("\n\n");
            rendered.push_str(diff);
        }
    }
    rendered
}
