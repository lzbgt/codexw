use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalApiCommand {
    StartSessionThread {
        session_id: String,
        client_id: Option<String>,
        lease_seconds: Option<u64>,
    },
    AttachSessionThread {
        session_id: String,
        thread_id: String,
        client_id: Option<String>,
        lease_seconds: Option<u64>,
    },
    StartTurn {
        session_id: String,
        prompt: String,
    },
    InterruptTurn {
        session_id: String,
    },
    StartShell {
        session_id: String,
        arguments: serde_json::Value,
    },
    SendShellInput {
        session_id: String,
        arguments: serde_json::Value,
    },
    TerminateShell {
        session_id: String,
        arguments: serde_json::Value,
    },
    UpdateService {
        session_id: String,
        arguments: serde_json::Value,
    },
    UpdateDependencies {
        session_id: String,
        arguments: serde_json::Value,
    },
    RenewAttachmentLease {
        session_id: String,
        client_id: Option<String>,
        lease_seconds: u64,
    },
    ReleaseAttachment {
        session_id: String,
        client_id: Option<String>,
    },
}

pub(crate) type SharedCommandQueue = Arc<Mutex<VecDeque<LocalApiCommand>>>;

pub(crate) fn new_command_queue() -> SharedCommandQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub(crate) fn enqueue_command(queue: &SharedCommandQueue, command: LocalApiCommand) -> Result<()> {
    if let Ok(mut guard) = queue.lock() {
        guard.push_back(command);
    }
    Ok(())
}

pub(super) fn drain_commands(queue: &SharedCommandQueue) -> Vec<LocalApiCommand> {
    match queue.lock() {
        Ok(mut guard) => guard.drain(..).collect(),
        Err(_) => Vec::new(),
    }
}
