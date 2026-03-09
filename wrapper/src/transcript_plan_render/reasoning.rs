use serde_json::Value;

pub(crate) fn format_plan(params: &Value) -> String {
    let explanation = params
        .get("explanation")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let plan = params.get("plan").and_then(Value::as_array);

    let mut lines = Vec::new();
    if let Some(explanation) = explanation {
        lines.push(explanation.to_string());
    }

    if let Some(plan) = plan {
        if plan.is_empty() {
            lines.push("(no steps provided)".to_string());
        } else {
            for step in plan {
                let step_text = step.get("step").and_then(Value::as_str).unwrap_or("-");
                let status = step
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("pending");
                let marker = match status {
                    "completed" => "✔",
                    "in_progress" => "□",
                    _ => "◦",
                };
                lines.push(format!("{marker} {step_text}"));
            }
        }
    }

    lines.join("\n")
}

pub(crate) fn render_reasoning_item(item: &Value) -> String {
    let summary = item
        .get("summary")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !summary.is_empty() {
        return summary.join("\n\n");
    }

    item.get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}
