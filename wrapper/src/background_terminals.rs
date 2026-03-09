#[path = "background_terminals/render.rs"]
mod render;
#[path = "background_terminals/tracking.rs"]
mod tracking;

pub(crate) use render::background_terminal_count;
pub(crate) use render::render_background_terminals;
pub(crate) use render::server_background_terminal_count;
pub(crate) use tracking::BackgroundTerminalSummary;
pub(crate) use tracking::clear_all_background_terminals;
pub(crate) use tracking::clear_completed_command_item;
pub(crate) use tracking::track_command_output_delta;
pub(crate) use tracking::track_started_command_item;
pub(crate) use tracking::track_terminal_interaction;
