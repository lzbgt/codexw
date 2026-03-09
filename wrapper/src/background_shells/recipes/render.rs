use super::super::BackgroundShellInteractionAction;
use super::super::BackgroundShellInteractionParameter;

pub(crate) fn render_recipe_parameters(
    parameters: &[BackgroundShellInteractionParameter],
) -> String {
    parameters
        .iter()
        .map(|parameter| {
            let mut rendered = parameter.name.clone();
            if parameter.required {
                rendered.push('*');
            }
            if let Some(default) = parameter.default.as_deref() {
                rendered.push_str(&format!("={default}"));
            }
            rendered
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn interaction_action_summary(action: &BackgroundShellInteractionAction) -> String {
    match action {
        BackgroundShellInteractionAction::Informational => "info".to_string(),
        BackgroundShellInteractionAction::Stdin {
            text,
            append_newline,
        } => {
            let mut summary = format!("stdin \"{}\"", summarize_recipe_text(text));
            if !append_newline {
                summary.push_str(" no-newline");
            }
            summary
        }
        BackgroundShellInteractionAction::Http {
            method,
            path,
            body,
            headers,
            expected_status,
        } => {
            let mut summary = format!("http {method} {path}");
            if !headers.is_empty() {
                summary.push_str(&format!(" headers={}", headers.len()));
            }
            if let Some(body) = body.as_deref() {
                summary.push_str(&format!(" body={}b", body.len()));
            }
            if let Some(expected_status) = expected_status {
                summary.push_str(&format!(" expect={expected_status}"));
            }
            summary
        }
        BackgroundShellInteractionAction::Tcp {
            payload,
            append_newline,
            expect_substring,
            read_timeout_ms,
        } => {
            let mut summary = "tcp".to_string();
            if let Some(payload) = payload.as_deref() {
                summary.push_str(&format!(" payload=\"{}\"", summarize_recipe_text(payload)));
                if *append_newline {
                    summary.push_str(" newline");
                }
            }
            if let Some(expect_substring) = expect_substring.as_deref() {
                summary.push_str(&format!(
                    " expect=\"{}\"",
                    summarize_recipe_text(expect_substring)
                ));
            }
            if let Some(timeout_ms) = read_timeout_ms {
                summary.push_str(&format!(" timeout={}ms", timeout_ms));
            }
            summary
        }
        BackgroundShellInteractionAction::Redis {
            command,
            expect_substring,
            read_timeout_ms,
        } => {
            let mut summary = format!("redis {}", command.join(" "));
            if let Some(expect_substring) = expect_substring.as_deref() {
                summary.push_str(&format!(
                    " expect=\"{}\"",
                    summarize_recipe_text(expect_substring)
                ));
            }
            if let Some(timeout_ms) = read_timeout_ms {
                summary.push_str(&format!(" timeout={}ms", timeout_ms));
            }
            summary
        }
    }
}

fn summarize_recipe_text(text: &str) -> String {
    const MAX_CHARS: usize = 40;
    let sanitized = text.replace('\n', "\\n");
    let mut chars = sanitized.chars();
    let summary = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}
