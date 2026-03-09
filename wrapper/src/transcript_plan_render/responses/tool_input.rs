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
