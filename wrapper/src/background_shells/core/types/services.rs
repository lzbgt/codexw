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
