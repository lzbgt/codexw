#[path = "services/render.rs"]
mod render;
#[path = "services/updates.rs"]
mod updates;

#[cfg(test)]
pub(crate) use self::render::parse_capability_issue_filter;
