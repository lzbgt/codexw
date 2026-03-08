use serde_json::Value;

#[path = "status_rate_credits.rs"]
mod status_rate_credits;
#[path = "status_rate_windows.rs"]
mod status_rate_windows;

pub(crate) fn render_rate_limit_lines(rate_limits: Option<&Value>) -> Vec<String> {
    let Some(rate_limits) = rate_limits else {
        return vec!["rate limits     unavailable".to_string()];
    };

    let mut lines = status_rate_windows::render_window_lines(rate_limits);
    let first_row = lines.is_empty();

    if let Some(credits) = rate_limits.get("credits")
        && let Some(credit_line) = status_rate_credits::render_credit_line(credits, first_row)
    {
        lines.push(credit_line);
    }

    if lines.is_empty() {
        vec!["rate limits     none reported".to_string()]
    } else {
        lines
    }
}
