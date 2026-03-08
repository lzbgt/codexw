use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use serde_json::json;

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

pub(crate) fn build_mcp_elicitation_response(params: &Value) -> Value {
    match params.get("mode").and_then(Value::as_str) {
        Some("form") => json!({
            "action": "accept",
            "content": build_mcp_elicitation_form_content(params.get("requestedSchema")),
            "_meta": Value::Null,
        }),
        Some("url") => json!({
            "action": "cancel",
            "content": Value::Null,
            "_meta": Value::Null,
        }),
        _ => json!({
            "action": "cancel",
            "content": Value::Null,
            "_meta": Value::Null,
        }),
    }
}

pub(crate) fn build_dynamic_tool_call_response(params: &Value) -> Value {
    let tool = params
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or("dynamic tool");
    let arguments = params
        .get("arguments")
        .map(render_dynamic_tool_arguments)
        .unwrap_or_else(|| "null".to_string());
    json!({
        "contentItems": [
            {
                "type": "inputText",
                "text": format!(
                    "codexw cannot execute client-side dynamic tool `{tool}` automatically; arguments={arguments}"
                )
            }
        ],
        "success": false
    })
}

fn build_mcp_elicitation_form_content(schema: Option<&Value>) -> Value {
    let Some(schema) = schema else {
        return Value::Object(Map::new());
    };
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let mut answers = Map::new();
    for (name, property_schema) in properties {
        let is_required = required.contains(name.as_str());
        if !is_required && property_schema.get("default").is_none() {
            continue;
        }
        if let Some(value) = build_mcp_field_value(&property_schema, is_required) {
            answers.insert(name, value);
        }
    }
    Value::Object(answers)
}

fn build_mcp_field_value(schema: &Value, required: bool) -> Option<Value> {
    if let Some(default) = schema.get("default") {
        return Some(default.clone());
    }
    if let Some(first) = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("const"))
    {
        return Some(first.clone());
    }
    if let Some(first) = schema
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    {
        return Some(first.clone());
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("boolean") if required => Some(Value::Bool(false)),
        Some("integer") if required => Some(integer_value(schema)),
        Some("number") if required => Some(number_value(schema)),
        Some("string") if required => Some(Value::String(string_value(schema))),
        Some("array") if required => Some(array_value(schema)),
        _ => None,
    }
}

fn integer_value(schema: &Value) -> Value {
    let minimum = schema.get("minimum").and_then(Value::as_i64).unwrap_or(0);
    let value = schema
        .get("maximum")
        .and_then(Value::as_i64)
        .map(|maximum| minimum.min(maximum))
        .unwrap_or(minimum);
    Value::Number(Number::from(value))
}

fn number_value(schema: &Value) -> Value {
    let minimum = schema.get("minimum").and_then(Value::as_f64).unwrap_or(0.0);
    let value = schema
        .get("maximum")
        .and_then(Value::as_f64)
        .map(|maximum| minimum.min(maximum))
        .unwrap_or(minimum);
    Value::Number(Number::from_f64(value).unwrap_or_else(|| Number::from(0)))
}

fn string_value(schema: &Value) -> String {
    let min_length = schema.get("minLength").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max_length = schema
        .get("maxLength")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let mut value = match schema.get("format").and_then(Value::as_str) {
        Some("email") => "user@example.com".to_string(),
        Some("uri") => "https://example.com".to_string(),
        Some("date") => "2026-03-08".to_string(),
        Some("date-time") => "2026-03-08T00:00:00Z".to_string(),
        _ if min_length == 0 => String::new(),
        _ => "x".repeat(min_length),
    };
    if value.len() < min_length {
        value.push_str(&"x".repeat(min_length - value.len()));
    }
    if let Some(max_length) = max_length {
        value.truncate(max_length);
    }
    value
}

fn array_value(schema: &Value) -> Value {
    if let Some(default) = schema.get("default") {
        return default.clone();
    }
    let min_items = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0);
    let mut values = Vec::new();
    if min_items > 0
        && let Some(item) = schema.get("items").and_then(|items| {
            items
                .get("oneOf")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("const"))
                .or_else(|| {
                    items
                        .get("enum")
                        .and_then(Value::as_array)
                        .and_then(|choices| choices.first())
                })
        })
    {
        values.push(item.clone());
    }
    Value::Array(values)
}

fn render_dynamic_tool_arguments(arguments: &Value) -> String {
    match arguments {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(arguments).unwrap_or_else(|_| "null".to_string()),
    }
}
