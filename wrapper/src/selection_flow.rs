#[path = "selection_flow/control.rs"]
mod control;
#[path = "selection_flow/model.rs"]
mod model;
#[path = "selection_flow/options.rs"]
mod options;
#[path = "selection_flow/status.rs"]
mod status;

pub(crate) use self::control::apply_model_choice;
pub(crate) use self::control::apply_permission_preset;
pub(crate) use self::control::apply_theme_choice;
pub(crate) use self::control::open_model_picker;
pub(crate) use self::control::open_permissions_picker;
pub(crate) use self::control::open_personality_picker;
pub(crate) use self::control::open_reasoning_picker;
pub(crate) use self::control::open_theme_picker;
pub(crate) use self::control::toggle_fast_mode;
pub(crate) use self::status::handle_pending_selection;
pub(crate) use self::status::pending_selection_status;

struct PermissionPreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    approval_policy: &'static str,
    thread_sandbox_mode: &'static str,
}

const PERMISSION_PRESETS: &[PermissionPreset] = &[
    PermissionPreset {
        id: "read-only",
        label: "Read Only",
        description: "Read files only; ask before edits or network access.",
        approval_policy: "on-request",
        thread_sandbox_mode: "read-only",
    },
    PermissionPreset {
        id: "auto",
        label: "Default",
        description: "Workspace writes allowed; ask before network or outside-workspace edits.",
        approval_policy: "on-request",
        thread_sandbox_mode: "workspace-write",
    },
    PermissionPreset {
        id: "full-access",
        label: "Full Access",
        description: "No approval prompts; unrestricted filesystem and network access.",
        approval_policy: "never",
        thread_sandbox_mode: "danger-full-access",
    },
];

const PERSONALITY_CHOICES: &[(&str, &str)] = &[
    ("default", "Use the backend default personality."),
    ("none", "No extra personality instructions."),
    ("friendly", "Warm, collaborative, and helpful."),
    ("pragmatic", "Concise, task-focused, and direct."),
];
