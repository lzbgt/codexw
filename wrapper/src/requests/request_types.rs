use crate::collaboration_apply::CollaborationModeAction;
use crate::model_personality_actions::ModelsAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreadListView {
    Threads,
    Agents,
}

#[derive(Debug)]
pub(crate) enum PendingRequest {
    Initialize,
    LoadApps,
    LoadSkills,
    LoadAccount,
    LogoutAccount,
    UploadFeedback {
        classification: String,
    },
    LoadRateLimits,
    LoadModels {
        action: ModelsAction,
    },
    LoadConfig,
    LoadMcpServers,
    LoadExperimentalFeatures,
    WindowsSandboxSetupStart {
        mode: String,
    },
    LoadCollaborationModes {
        action: CollaborationModeAction,
    },
    ListThreads {
        search_term: Option<String>,
        cwd_filter: Option<String>,
        source_kinds: Option<Vec<String>>,
        view: ThreadListView,
    },
    StartThread {
        initial_prompt: Option<String>,
    },
    ResumeThread {
        initial_prompt: Option<String>,
    },
    ForkThread {
        initial_prompt: Option<String>,
    },
    CompactThread,
    RenameThread {
        name: String,
    },
    CleanBackgroundTerminals,
    StartRealtime {
        prompt: String,
    },
    AppendRealtimeText {
        text: String,
    },
    StopRealtime,
    StartReview {
        target_description: String,
    },
    StartTurn {
        auto_generated: bool,
    },
    SteerTurn {
        display_text: String,
    },
    InterruptTurn,
    ExecCommand {
        process_id: String,
        command: String,
    },
    TerminateExecCommand {
        process_id: String,
    },
    FuzzyFileSearch {
        query: String,
    },
}
