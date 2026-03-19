use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::history_render::render_resumed_history;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_turn_start;
use crate::state::AppState;
use crate::state::get_string;

fn send_initial_thread_prompt(
    text: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    writer: &mut ChildStdin,
    thread_id: String,
) -> Result<()> {
    let (local_images, remote_images) = state.take_pending_attachments();
    let submission = build_turn_input(
        text,
        resolved_cwd,
        &local_images,
        &remote_images,
        &state.apps,
        &state.plugins,
        &state.skills,
    );
    send_turn_start(
        writer,
        state,
        cli,
        resolved_cwd,
        thread_id,
        submission,
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::process::Stdio;

    use serde_json::json;

    use super::handle_resumed_thread;
    use crate::Cli;
    use crate::output::Output;
    use crate::state::AppState;

    fn test_cli() -> Cli {
        crate::runtime_process::normalize_cli(Cli {
            codex_bin: "codex".to_string(),
            config_overrides: Vec::new(),
            enable_features: Vec::new(),
            disable_features: Vec::new(),
            resume: None,
            resume_picker: false,
            cwd: None,
            model: None,
            model_provider: None,
            auto_continue: true,
            verbose_events: false,
            verbose_thinking: true,
            raw_json: false,
            no_experimental_api: false,
            yolo: false,
            local_api: false,
            local_api_bind: "127.0.0.1:0".to_string(),
            local_api_token: None,
            prompt: Vec::new(),
        })
    }

    fn spawn_recording_stdin() -> (
        tempfile::TempDir,
        std::process::Child,
        std::process::ChildStdin,
        std::path::PathBuf,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("requests.jsonl");
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("cat > \"$1\"")
            .arg("sh")
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn recorder");
        let stdin = child.stdin.take().expect("stdin");
        (temp, child, stdin, path)
    }

    fn read_recorded_requests(
        child: &mut std::process::Child,
        writer: std::process::ChildStdin,
        path: &std::path::Path,
    ) -> Vec<serde_json::Value> {
        drop(writer);
        child.wait().expect("wait recorder");
        let contents = std::fs::read_to_string(path).expect("read requests");
        contents
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("parse request"))
            .collect()
    }

    #[test]
    fn resumed_initial_prompt_includes_queued_attachments() {
        let cli = test_cli();
        let mut state = AppState::new(true, false);
        state.pending_local_images.push("/tmp/cat.png".to_string());
        state
            .pending_remote_images
            .push("https://example.com/dog.png".to_string());
        let mut output = Output::default();
        let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

        handle_resumed_thread(
            &json!({
                "thread": {
                    "id": "thread-42",
                    "path": "/tmp/thread-42"
                }
            }),
            &cli,
            "/tmp/project",
            &mut state,
            &mut output,
            &mut writer,
            Some("continue with images"),
        )
        .expect("handle resumed thread");

        let requests = read_recorded_requests(&mut child, writer, &path);
        let request = requests.last().expect("turn/start request");
        assert_eq!(request["method"], json!("turn/start"));
        assert_eq!(
            request["params"]["input"],
            json!([
                {
                    "type": "image",
                    "url": "https://example.com/dog.png"
                },
                {
                    "type": "localImage",
                    "path": "/tmp/cat.png"
                },
                {
                    "type": "text",
                    "text": "continue with images",
                    "text_elements": []
                }
            ])
        );
        assert!(state.pending_local_images.is_empty());
        assert!(state.pending_remote_images.is_empty());
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_loaded_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
    status_label: &str,
    thread_field_context: &'static str,
    render_history: bool,
) -> Result<()> {
    state.pending_thread_switch = false;
    state.reset_thread_context();
    let thread_id = get_string(result, &["thread", "id"])
        .context(thread_field_context)?
        .to_string();
    state.current_rollout_path =
        get_string(result, &["thread", "path"]).map(std::path::PathBuf::from);
    state.thread_id = Some(thread_id.clone());
    output.line_stderr(format!("[thread] {status_label} {thread_id}"))?;
    if render_history {
        render_resumed_history(result, state, output)?;
    }
    if let Some(text) = initial_prompt {
        send_initial_thread_prompt(text, cli, resolved_cwd, state, writer, thread_id)?;
    }
    Ok(())
}

pub(crate) fn handle_started_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "started",
        "thread/start missing thread.id",
        false,
    )
}

pub(crate) fn handle_resumed_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "resumed",
        "thread/resume missing thread.id",
        true,
    )
}

pub(crate) fn handle_forked_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "forked to",
        "thread/fork missing thread.id",
        true,
    )
}
