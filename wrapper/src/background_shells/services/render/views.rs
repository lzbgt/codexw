#[path = "views/capabilities.rs"]
mod capabilities;
#[path = "views/services.rs"]
mod services;

pub(super) use self::services::parse_service_issue_filter;
