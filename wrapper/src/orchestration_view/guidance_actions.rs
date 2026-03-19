use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellInteractionParameter;
use crate::background_shells::BackgroundShellInteractionRecipe;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::state::AppState;

use super::DependencyFilter;
use super::DependencySelection;
use super::pluralize;
use super::render_orchestration_dependencies;

#[path = "guidance_actions/actions.rs"]
mod actions;
#[path = "guidance_actions/guidance.rs"]
mod guidance;
#[path = "guidance_actions/render.rs"]
mod render;
#[path = "guidance_actions/shared.rs"]
mod shared;

use actions::action_lines;
use actions::action_lines_for_capability;
use guidance::guidance_lines;
use guidance::guidance_lines_for_capability;
#[cfg(test)]
use guidance::guidance_lines_for_tool;
#[cfg(test)]
use guidance::guidance_lines_for_tool_capability;
pub(crate) use render::orchestration_next_action_summary;
#[cfg(test)]
pub(crate) use render::orchestration_next_action_summary_for_tool;
pub(crate) use render::render_orchestration_actions;
pub(crate) use render::render_orchestration_actions_for_capability;
#[cfg(test)]
pub(crate) use render::render_orchestration_actions_for_tool;
#[cfg(test)]
pub(crate) use render::render_orchestration_actions_for_tool_capability;
pub(crate) use render::render_orchestration_blockers_for_capability;
pub(crate) use render::render_orchestration_guidance;
pub(crate) use render::render_orchestration_guidance_for_capability;
#[cfg(test)]
pub(crate) use render::render_orchestration_guidance_for_tool;
#[cfg(test)]
pub(crate) use render::render_orchestration_guidance_for_tool_capability;
pub(super) use shared::ActionAudience;
pub(super) use shared::first_blocking_ref_for_capability;
pub(super) use shared::first_provider_ref_for_capability;
pub(super) use shared::normalize_capability_ref;
pub(super) use shared::operator_recipe_command;
pub(super) use shared::tool_recipe_call;
pub(super) use shared::unique_running_service_ref;
pub(super) use shared::unique_service_recipe_name_by_readiness;
pub(super) use shared::unique_service_ref_by_readiness;
pub(super) use shared::unique_shell_ref_by_intent;

#[cfg(test)]
pub(crate) use render::orchestration_guidance_summary;
