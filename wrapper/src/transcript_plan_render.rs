use serde_json::Value;
use serde_json::json;

pub(crate) fn format_plan(params: &Value) -> String {
    params
        .get("plan")
        .and_then(Value::as_array)
        .map(|plan| {
            plan.iter()
                .map(|step| {
                    let step_text = step.get("step").and_then(Value::as_str).unwrap_or("-");
                    let status = step
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending");
                    format!("- [{status}] {step_text}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
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

pub(crate) fn build_tool_user_input_response(params: &Value) -> Value {
    let mut answers = serde_json::Map::new();
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        for question in questions {
            let Some(id) = question.get("id").and_then(Value::as_str) else {
                continue;
            };
            let selected = question
                .get("options")
                .and_then(Value::as_array)
                .and_then(|options| options.first())
                .and_then(|first| first.get("label"))
                .and_then(Value::as_str)
                .map(|label| vec![label.to_string()])
                .unwrap_or_else(|| vec!["".to_string()]);
            answers.insert(id.to_string(), json!({ "answers": selected }));
        }
    }
    Value::Object(
        [("answers".to_string(), Value::Object(answers))]
            .into_iter()
            .collect(),
    )
}
