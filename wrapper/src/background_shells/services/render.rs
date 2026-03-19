#[path = "render/index.rs"]
mod index;
#[path = "render/views.rs"]
mod views;

#[cfg(test)]
pub(crate) use self::index::parse_capability_issue_filter;
