use crate::collaboration_apply::CollaborationModeAction;
use crate::model_personality_actions::ModelsAction;

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
    LoadCollaborationModes {
        action: CollaborationModeAction,
    },
    ListThreads {
        search_term: Option<String>,
        cwd_filter: Option<String>,
        allow_fallback_all: bool,
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
