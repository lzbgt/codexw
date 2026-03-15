#[path = "control/attachment.rs"]
mod attachment;
#[path = "control/process.rs"]
mod process;
#[path = "control/queue.rs"]
mod queue;

use anyhow::Result;
use std::process::ChildStdin;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;

use super::SharedSnapshot;

pub(crate) use queue::LocalApiCommand;
pub(crate) use queue::SharedCommandQueue;
pub(crate) use queue::enqueue_command;
pub(crate) use queue::new_command_queue;

pub(crate) fn process_local_api_commands(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    snapshot: &SharedSnapshot,
    queue: &SharedCommandQueue,
) -> Result<()> {
    process::process_local_api_commands(cli, resolved_cwd, state, output, writer, snapshot, queue)
}
