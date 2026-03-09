use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

pub(crate) const DEFAULT_POLL_LIMIT: usize = 40;
pub(crate) const MAX_POLL_LIMIT: usize = 200;
pub(crate) const MAX_STORED_LINES: usize = 2_000;
pub(crate) const DEFAULT_READY_WAIT_TIMEOUT_MS: u64 = 5_000;
pub(crate) const READY_WAIT_POLL_INTERVAL_MS: u64 = 25;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum BackgroundShellIntent {
    Prerequisite,
    #[default]
    Observation,
    Service,
}

impl BackgroundShellIntent {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Prerequisite => "prerequisite",
            Self::Observation => "observation",
            Self::Service => "service",
        }
    }

    pub(crate) fn is_blocking(self) -> bool {
        matches!(self, Self::Prerequisite)
    }

    pub(crate) fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "prerequisite" => Some(Self::Prerequisite),
            "observation" => Some(Self::Observation),
            "service" => Some(Self::Service),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellServiceReadiness {
    Booting,
    Ready,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellReadyWaitOutcome {
    AlreadyReady,
    BecameReady { waited_ms: u64 },
}

impl BackgroundShellServiceReadiness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Booting => "booting",
            Self::Ready => "ready",
            Self::Untracked => "untracked",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BackgroundShellOrigin {
    pub(crate) source_thread_id: Option<String>,
    pub(crate) source_call_id: Option<String>,
    pub(crate) source_tool: Option<String>,
}

#[derive(Clone, Default)]
pub(crate) struct BackgroundShellManager {
    pub(crate) inner: Arc<BackgroundShellManagerInner>,
}

#[derive(Default)]
pub(crate) struct BackgroundShellManagerInner {
    pub(crate) next_job_id: AtomicU64,
    pub(crate) jobs: Mutex<HashMap<String, Arc<Mutex<BackgroundShellJobState>>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackgroundShellJobSnapshot {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) command: String,
    pub(crate) cwd: String,
    pub(crate) intent: BackgroundShellIntent,
    pub(crate) label: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) service_capabilities: Vec<String>,
    pub(crate) dependency_capabilities: Vec<String>,
    pub(crate) service_protocol: Option<String>,
    pub(crate) service_endpoint: Option<String>,
    pub(crate) attach_hint: Option<String>,
    pub(crate) interaction_recipes: Vec<BackgroundShellInteractionRecipe>,
    pub(crate) ready_pattern: Option<String>,
    pub(crate) service_readiness: Option<BackgroundShellServiceReadiness>,
    pub(crate) origin: BackgroundShellOrigin,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) total_lines: u64,
    pub(crate) recent_lines: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct BackgroundShellJobState {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) command: String,
    pub(crate) cwd: String,
    pub(crate) intent: BackgroundShellIntent,
    pub(crate) label: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) service_capabilities: Vec<String>,
    pub(crate) dependency_capabilities: Vec<String>,
    pub(crate) service_protocol: Option<String>,
    pub(crate) service_endpoint: Option<String>,
    pub(crate) attach_hint: Option<String>,
    pub(crate) interaction_recipes: Vec<BackgroundShellInteractionRecipe>,
    pub(crate) ready_pattern: Option<String>,
    pub(crate) service_ready: bool,
    pub(crate) origin: BackgroundShellOrigin,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) status: BackgroundShellJobStatus,
    pub(crate) total_lines: u64,
    pub(crate) lines: VecDeque<BackgroundShellOutputLine>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackgroundShellOutputLine {
    pub(crate) cursor: u64,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) enum BackgroundShellJobStatus {
    Running,
    Completed(i32),
    Failed(String),
    Terminated(Option<i32>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellInteractionRecipe {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) example: Option<String>,
    pub(crate) parameters: Vec<BackgroundShellInteractionParameter>,
    pub(crate) action: BackgroundShellInteractionAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellInteractionParameter {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) default: Option<String>,
    pub(crate) required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BackgroundShellInteractionAction {
    Informational,
    Stdin {
        text: String,
        append_newline: bool,
    },
    Http {
        method: String,
        path: String,
        body: Option<String>,
        headers: Vec<(String, String)>,
        expected_status: Option<u16>,
    },
    Tcp {
        payload: Option<String>,
        append_newline: bool,
        expect_substring: Option<String>,
        read_timeout_ms: Option<u64>,
    },
    Redis {
        command: Vec<String>,
        expect_substring: Option<String>,
        read_timeout_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellCapabilityDependencyState {
    Satisfied,
    Booting,
    Missing,
    Ambiguous,
}

impl BackgroundShellCapabilityDependencyState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Booting => "booting",
            Self::Missing => "missing",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellCapabilityIssueClass {
    Healthy,
    Missing,
    Booting,
    Untracked,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellServiceIssueClass {
    Ready,
    Booting,
    Untracked,
    Conflicts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellCapabilityDependencySummary {
    pub(crate) job_id: String,
    pub(crate) job_alias: Option<String>,
    pub(crate) job_label: Option<String>,
    pub(crate) capability: String,
    pub(crate) blocking: bool,
    pub(crate) status: BackgroundShellCapabilityDependencyState,
    pub(crate) providers: Vec<String>,
}
