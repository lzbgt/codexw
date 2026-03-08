use serde_json::Value;

use crate::state::get_string;
use crate::transcript_item_summary::summarize_file_change_paths;

pub(crate) fn render_command_completion(
    command: &str,
    status: &str,
    exit_code: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("$ {command}");
    if !is_successful_command_completion(status, exit_code) {
        rendered.push_str(&format!("\nstatus  {status}\nexit    {exit_code}"));
    }
    if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

fn is_successful_command_completion(status: &str, exit_code: &str) -> bool {
    status == "completed" && exit_code == "0"
}

pub(crate) fn render_local_command_completion(
    command: &str,
    exit_code: &str,
    stdout: &str,
    stderr: &str,
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
    }
    if show_stdout {
        rendered.push_str("\n\n");
        if show_stream_labels {
            rendered.push_str("[stdout]\n");
        }
        rendered.push_str(stdout);
    }
    if show_stderr {
        rendered.push_str("\n\n");
        if show_stream_labels {
            rendered.push_str("[stderr]\n");
        }
        rendered.push_str(stderr);
    }
    rendered
}

pub(crate) fn render_file_change_completion(
    item: &Value,
    status: &str,
    output: Option<&str>,
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
