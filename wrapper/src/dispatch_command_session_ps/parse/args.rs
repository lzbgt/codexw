use anyhow::Result;

use crate::background_shells::BackgroundShellInteractionRecipe;

pub(crate) fn parse_ps_send_args(raw_args: &str) -> Option<(&str, &str)> {
    let remainder = raw_args
        .trim_start()
        .strip_prefix("send")
        .or_else(|| raw_args.trim_start().strip_prefix("write"))
        .or_else(|| raw_args.trim_start().strip_prefix("stdin"))?
        .trim_start();
    let (reference, text) = remainder.split_once(char::is_whitespace)?;
    let text = text.trim_start();
    if reference.is_empty() || text.is_empty() {
        None
    } else {
        Some((reference, text))
    }
}

pub(crate) fn parse_ps_run_args(raw_args: &str) -> Option<(&str, &str, Option<&str>)> {
    let remainder = raw_args
        .trim_start()
        .strip_prefix("run")
        .or_else(|| raw_args.trim_start().strip_prefix("invoke"))?
        .trim_start();
    let (reference, remainder) = remainder.split_once(char::is_whitespace)?;
    let remainder = remainder.trim_start();
    let (recipe, arg_tail) = match remainder.split_once(char::is_whitespace) {
        Some((recipe, tail)) => (recipe, Some(tail.trim_start())),
        None => (remainder, None),
    };
    if reference.is_empty() || recipe.is_empty() {
        None
    } else {
        Some((
            reference,
            recipe,
            arg_tail.filter(|value| !value.is_empty()),
        ))
    }
}

pub(crate) fn parse_ps_relabel_args(raw_args: &str) -> Option<(&str, Option<String>)> {
    let remainder = raw_args.trim_start().strip_prefix("relabel")?.trim_start();
    let (reference, label) = remainder.split_once(char::is_whitespace)?;
    let label = label.trim_start();
    if reference.is_empty() || label.is_empty() {
        return None;
    }
    let normalized = if matches!(label, "none" | "-") {
        None
    } else {
        Some(label.to_string())
    };
    Some((reference, normalized))
}

pub(crate) fn parse_ps_contract_args(
    raw_args: &str,
) -> Option<(&str, serde_json::Map<String, serde_json::Value>)> {
    let remainder = raw_args.trim_start().strip_prefix("contract")?.trim_start();
    let (reference, contract) = remainder.split_once(char::is_whitespace)?;
    let contract = contract.trim_start();
    if reference.is_empty() || contract.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(contract).ok()?;
    let object = value.as_object()?.clone();
    Some((reference, object))
}

pub(crate) fn parse_optional_contract_field(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<Option<Option<String>>, String> {
    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(Some(None)),
        Some(serde_json::Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Err(format!(":ps contract `{field_name}` cannot be empty"))
            } else {
                Ok(Some(Some(trimmed.to_string())))
            }
        }
        Some(_) => Err(format!(
            ":ps contract `{field_name}` must be a string or null"
        )),
    }
}

pub(crate) fn parse_optional_contract_recipes(
    value: Option<&serde_json::Value>,
) -> Result<Option<Vec<BackgroundShellInteractionRecipe>>, String> {
    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(Some(Vec::new())),
        Some(value) => {
            crate::background_shells::parse_background_shell_interaction_recipes(Some(value))
                .map(Some)
                .map_err(|err| format!(":ps contract {err}"))
        }
    }
}

pub(crate) fn parse_operator_recipe_args(
    raw_args: Option<&str>,
) -> Result<std::collections::HashMap<String, String>> {
    let Some(raw_args) = raw_args else {
        return Ok(std::collections::HashMap::new());
    };
    let value: serde_json::Value = serde_json::from_str(raw_args)
        .map_err(|err| anyhow::anyhow!("recipe args must be valid JSON object text: {err}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("recipe args must be a JSON object"))?;
    let mut args = std::collections::HashMap::with_capacity(object.len());
    for (key, value) in object {
        let rendered = match value {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Bool(flag) => flag.to_string(),
            serde_json::Value::Number(number) => number.to_string(),
            _ => {
                return Err(anyhow::anyhow!(
                    "recipe arg `{key}` must be a string, number, or boolean"
                ));
            }
        };
        args.insert(key.clone(), rendered);
    }
    Ok(args)
}

pub(crate) fn parse_ps_wait_timeout(raw: Option<&str>) -> Result<u64> {
    let Some(raw) = raw else {
        return Ok(5_000);
    };
    raw.parse::<u64>()
        .map_err(|_| anyhow::anyhow!("timeoutMs must be a non-negative integer"))
}

pub(crate) fn parse_ps_capability_list(args: &[&str], usage: &str) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Err(format!("usage: {usage}"));
    }
    if args.len() == 1 && matches!(args[0], "none" | "-") {
        return Ok(Vec::new());
    }
    let mut capabilities = Vec::with_capacity(args.len());
    for raw in args {
        let Some(capability) = raw.strip_prefix('@') else {
            return Err(format!("usage: {usage}"));
        };
        if capability.is_empty() || !super::is_valid_capability_ref(capability) {
            return Err(format!("usage: {usage}"));
        }
        capabilities.push(capability.to_string());
    }
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}

pub(crate) fn parse_ps_provide_capabilities(args: &[&str]) -> Result<Vec<String>, String> {
    parse_ps_capability_list(
        args,
        ":ps provide <jobId|alias|@capability|n> <@capability...|none>",
    )
}
