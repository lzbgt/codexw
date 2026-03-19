use super::*;

#[cfg(test)]
pub(crate) fn orchestration_guidance_summary(state: &AppState) -> Option<String> {
    guidance_lines(state).first().cloned()
}

pub(crate) fn orchestration_next_action_summary(state: &AppState) -> Option<String> {
    action_lines(state, ActionAudience::Operator)
        .first()
        .cloned()
}

#[cfg(test)]
pub(crate) fn orchestration_next_action_summary_for_tool(state: &AppState) -> Option<String> {
    action_lines(state, ActionAudience::Tool).first().cloned()
}

pub(crate) fn render_orchestration_guidance(state: &AppState) -> String {
    let lines = guidance_lines(state);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Next action:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_guidance_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = guidance_lines_for_capability(state, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Next action (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

#[cfg(test)]
pub(crate) fn render_orchestration_guidance_for_tool(state: &AppState) -> String {
    let lines = guidance_lines_for_tool(state);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Next action:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

#[cfg(test)]
pub(crate) fn render_orchestration_guidance_for_tool_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = guidance_lines_for_tool_capability(state, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Next action (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_blockers_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    Ok(render_orchestration_dependencies(
        state,
        &DependencySelection {
            filter: DependencyFilter::Blocking,
            capability: Some(capability),
        },
    ))
}

pub(crate) fn render_orchestration_actions(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Operator);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_actions_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Operator, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

#[cfg(test)]
pub(crate) fn render_orchestration_actions_for_tool(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Tool);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

#[cfg(test)]
pub(crate) fn render_orchestration_actions_for_tool_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Tool, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}
