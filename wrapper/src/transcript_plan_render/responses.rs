use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use serde_json::json;

pub(crate) fn build_tool_user_input_response(params: &Value) -> Value {
    let mut answers = serde_json::Map::new();
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        for question in questions {
            let Some(id) = question.get("id").and_then(Value::as_str) else {
                continue;
            };
            let selected = vec![select_tool_user_input_option(question)];
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

fn select_tool_user_input_option(question: &Value) -> String {
    let Some(options) = question.get("options").and_then(Value::as_array) else {
        return String::new();
    };

    options
        .iter()
        .max_by_key(|option| tool_user_input_option_score(option))
        .and_then(|option| option.get("label"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn tool_user_input_option_score(option: &Value) -> i32 {
    let label = option
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let description = option
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let combined = format!("{label} {description}");

    let mut score = 0;
    for positive in [
        "allow", "accept", "approve", "continue", "enable", "grant", "ok", "open", "proceed",
        "run", "trust", "yes",
    ] {
        if combined.contains(positive) {
            score += 2;
        }
    }
    for negative in [
        "cancel", "decline", "deny", "disable", "no", "reject", "skip", "stop",
    ] {
        if combined.contains(negative) {
            score -= 2;
        }
    }
    score
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
