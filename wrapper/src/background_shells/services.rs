#[path = "services/render.rs"]
mod render;
#[path = "services/updates.rs"]
mod updates;

pub(crate) use self::render::parse_capability_issue_filter;
