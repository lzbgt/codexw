use serde_json::json;

use crate::dispatch_command_session_meta::INIT_PROMPT;
use crate::dispatch_command_session_meta::current_rollout_message;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::model_catalog::extract_models;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::ThreadListView;
use crate::requests::send_windows_sandbox_setup_start;
use crate::state::AppState;
use crate::state::PendingSelection;

#[path = "main_test_session_selections/harness.rs"]
mod harness;
#[path = "main_test_session_selections/meta.rs"]
mod meta;
#[path = "main_test_session_selections/pickers.rs"]
mod pickers;
#[path = "main_test_session_selections/threads.rs"]
mod threads;

use harness::build_cli;
use harness::config_contents;
use harness::read_recorded_requests;
use harness::spawn_recording_stdin;
use harness::spawn_sink_stdin;
use harness::test_codex_home;
