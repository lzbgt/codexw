use crate::Cli;
use crate::app_input_editor::handle_submit;
use crate::app_input_editor::try_complete_file_token;
use crate::app_input_interrupt::handle_ctrl_c;
use crate::app_input_interrupt::handle_escape;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::prompt_is_visible;
use crate::requests::PendingRequest;
use crate::runtime_keys::InputKey;
use crate::state::AppState;
use std::process::Command;
use std::process::Stdio;

#[path = "main_test_runtime_prompt/completion.rs"]
mod completion;
#[path = "main_test_runtime_prompt/submit.rs"]
mod submit;

fn spawn_sink_stdin() -> std::process::ChildStdin {
    Command::new("sh")
        .arg("-c")
        .arg("cat >/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sink")
        .stdin
        .take()
        .expect("stdin")
}
