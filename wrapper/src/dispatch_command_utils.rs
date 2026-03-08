use std::process::Command;
use std::process::Stdio;

use anyhow::Result;

use crate::commands::builtin_command_names;
use crate::output::Output;

pub(crate) struct FeedbackCommand {
    pub(crate) classification: String,
    pub(crate) reason: Option<String>,
    pub(crate) include_logs: bool,
}

pub(crate) fn parse_feedback_args(args: &[String]) -> Option<FeedbackCommand> {
    if args.is_empty() {
        return None;
    }
    let mut include_logs = false;
    let mut filtered = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--logs" => include_logs = true,
            "--no-logs" => include_logs = false,
            _ => filtered.push(arg.as_str()),
        }
    }
    let Some(first) = filtered.first() else {
        return None;
    };
    let classification = normalize_feedback_classification(first)?;
    let reason = join_prompt(
        &filtered[1..]
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>(),
    );
    Some(FeedbackCommand {
        classification,
        reason,
        include_logs,
    })
}

pub(crate) fn join_prompt(parts: &[String]) -> Option<String> {
    let joined = parts.join(" ").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

pub(crate) fn is_builtin_command(command_line: &str) -> bool {
    let command = command_line.split_whitespace().next().unwrap_or_default();
    matches!(command, "h" | "q") || builtin_command_names().contains(&command)
}

pub(crate) fn copy_to_clipboard(text: &str, output: &mut Output) -> Result<()> {
    if cfg!(target_os = "macos") {
        let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            output.block_stdout("Copied text", text)?;
            return Ok(());
        };
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write as _;
            stdin.write_all(text.as_bytes())?;
        }
        let _ = child.wait();
        output.line_stderr("[copy] copied last assistant reply to clipboard")?;
    } else {
        output.block_stdout("Copied text", text)?;
    }
    Ok(())
}

fn normalize_feedback_classification(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bug" => Some("bug".to_string()),
        "bad" | "bad-result" | "bad_result" => Some("bad_result".to_string()),
        "good" | "good-result" | "good_result" => Some("good_result".to_string()),
        "safety" | "safety-check" | "safety_check" => Some("safety_check".to_string()),
        "other" => Some("other".to_string()),
        _ => None,
    }
}
