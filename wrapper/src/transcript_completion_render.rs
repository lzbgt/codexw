use serde_json::Value;

use crate::state::get_string;
use crate::transcript_item_summary::summarize_file_change_paths;

pub(crate) fn render_command_completion(
    command: &str,
    status: &str,
    exit_code: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("{command}\n[status] {status}  [exit] {exit_code}");
    if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

pub(crate) fn render_local_command_completion(
    command: &str,
    exit_code: &str,
    stdout: &str,
    stderr: &str,
) -> String {
    let mut rendered = format!("{command}\n[exit] {exit_code}");
    if !stdout.trim().is_empty() {
        rendered.push_str("\n\n[stdout]\n");
        rendered.push_str(stdout.trim_end());
    }
    if !stderr.trim().is_empty() {
        rendered.push_str("\n\n[stderr]\n");
        rendered.push_str(stderr.trim_end());
    }
    rendered
}

pub(crate) fn render_file_change_completion(
    item: &Value,
    status: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("[status] {status}\n{}", summarize_file_change_paths(item));
    let structured = render_file_changes(item);
    if !structured.is_empty() {
        rendered.push_str("\n\n");
        rendered.push_str(&structured);
    } else if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
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
