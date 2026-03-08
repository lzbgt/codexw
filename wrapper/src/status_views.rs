use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::state::get_string;

pub(crate) fn render_permissions_snapshot(cli: &Cli) -> String {
    [
        format!("approval policy  {}", approval_policy(cli)),
        format!("thread sandbox   {}", thread_sandbox_mode(cli)),
        format!(
            "turn sandbox     {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        "network access    enabled".to_string(),
        "tool use          automatic".to_string(),
        "shell exec        automatic".to_string(),
        "host access       full".to_string(),
    ]
    .join("\n")
}

pub(crate) fn render_config_snapshot(result: &Value) -> String {
    if result.is_null() {
        return "config unavailable".to_string();
    }
    serde_json::to_string_pretty(result).unwrap_or_else(|_| summarize_value(result))
}

pub(crate) fn summarize_sandbox_policy(policy: &Value) -> String {
    match get_string(policy, &["type"]).unwrap_or("unknown") {
        "dangerFullAccess" => "dangerFullAccess".to_string(),
        other => summarize_value(&json!({
            "type": other,
            "policy": policy,
        })),
    }
}

pub(crate) fn render_account_summary(account: Option<&Value>) -> Option<String> {
    let account = account?;
    if account.is_null() {
        return Some("not signed in".to_string());
    }
    let account_type = get_string(account, &["type"])
        .or_else(|| get_string(account, &["authMode"]))
        .unwrap_or("unknown");
    let mut parts = vec![account_type.to_string()];
    if let Some(email) = get_string(account, &["email"]) {
        parts.push(email.to_string());
    }
    if let Some(plan_type) = get_string(account, &["planType"]) {
        parts.push(format!("plan={plan_type}"));
    }
    Some(parts.join(" "))
}

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

pub(crate) fn render_token_usage_summary(token_usage: Option<&Value>) -> Option<String> {
    let token_usage = token_usage?;
    let last_total = token_usage
        .get("last")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    let cumulative_total = token_usage
        .get("total")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    match (last_total, cumulative_total) {
        (Some(last_total), Some(cumulative_total)) => {
            Some(format!("last={} total={}", last_total, cumulative_total))
        }
        (Some(last_total), None) => Some(format!("last={last_total}")),
        (None, Some(cumulative_total)) => Some(format!("total={cumulative_total}")),
        (None, None) => None,
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

pub(crate) fn summarize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => string.to_string(),
        Value::Array(array) => {
            if array.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", array.len())
            }
        }
        Value::Object(object) => object
            .iter()
            .take(6)
            .map(|(key, value)| format!("{key}={}", summarize_value(value)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}
