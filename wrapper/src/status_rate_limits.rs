use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use serde_json::Value;

pub(crate) fn render_rate_limit_lines(rate_limits: Option<&Value>) -> Vec<String> {
    let Some(rate_limits) = rate_limits else {
        return vec!["rate limits     unavailable".to_string()];
    };

    let mut lines = Vec::new();
    let mut first_row = true;
    for (label, window_key) in [("primary", "primary"), ("secondary", "secondary")] {
        let Some(window) = rate_limits.get(window_key) else {
            continue;
        };
        if window.is_null() {
            continue;
        }
        let used_percent = window
            .get("usedPercent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let percent_left = (100.0 - used_percent).clamp(0.0, 100.0);
        let window_minutes = window.get("windowDurationMins").and_then(Value::as_i64);
        let duration_label = window_minutes
            .map(get_limits_duration)
            .unwrap_or_else(|| label.to_string());
        let reset_label = window
            .get("resetsAt")
            .and_then(Value::as_i64)
            .and_then(format_reset_timestamp_local);
        let mut line = format!(
            "{}{} limit {}",
            if first_row {
                "rate limits     "
            } else {
                "                "
            },
            duration_label,
            format_status_limit_summary(percent_left),
        );
        if let Some(reset_label) = reset_label {
            line.push_str(&format!(" (resets {reset_label})"));
        }
        lines.push(line);
        first_row = false;
    }

    if let Some(credits) = rate_limits.get("credits")
        && let Some(credit_line) = render_credit_line(credits, first_row)
    {
        lines.push(credit_line);
    }

    if lines.is_empty() {
        vec!["rate limits     none reported".to_string()]
    } else {
        lines
    }
}

fn render_credit_line(credits: &Value, first_row: bool) -> Option<String> {
    if !credits
        .get("hasCredits")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let prefix = if first_row {
        "rate limits     "
    } else {
        "                "
    };
    if credits
        .get("unlimited")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Some(format!("{prefix}credits unlimited"));
    }
    let balance = credits.get("balance").and_then(Value::as_str)?.trim();
    if balance.is_empty() {
        return None;
    }
    Some(format!("{prefix}credits {balance}"))
}

fn get_limits_duration(window_minutes: i64) -> String {
    const MINUTES_PER_HOUR: i64 = 60;
    const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
    const MINUTES_PER_WEEK: i64 = 7 * MINUTES_PER_DAY;
    const MINUTES_PER_MONTH: i64 = 30 * MINUTES_PER_DAY;
    const ROUNDING_BIAS_MINUTES: i64 = 3;

    let window_minutes = window_minutes.max(0);
    if window_minutes <= MINUTES_PER_DAY.saturating_add(ROUNDING_BIAS_MINUTES) {
        let adjusted = window_minutes.saturating_add(ROUNDING_BIAS_MINUTES);
        let hours = std::cmp::max(1, adjusted / MINUTES_PER_HOUR);
        format!("{hours}h")
    } else if window_minutes <= MINUTES_PER_WEEK.saturating_add(ROUNDING_BIAS_MINUTES) {
        "weekly".to_string()
    } else if window_minutes <= MINUTES_PER_MONTH.saturating_add(ROUNDING_BIAS_MINUTES) {
        "monthly".to_string()
    } else {
        "annual".to_string()
    }
}

fn format_status_limit_summary(percent_remaining: f64) -> String {
    format!("{percent_remaining:.0}% left")
}

fn format_reset_timestamp_local(unix_seconds: i64) -> Option<String> {
    let dt_utc = DateTime::<Utc>::from_timestamp(unix_seconds, 0)?;
    let dt_local = dt_utc.with_timezone(&Local);
    let now = Local::now();
    let time = dt_local.format("%H:%M").to_string();
    if dt_local.date_naive() == now.date_naive() {
        Some(time)
    } else {
        Some(format!("{time} on {}", dt_local.format("%-d %b")))
    }
}
