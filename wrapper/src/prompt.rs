pub fn parse_auto_mode_stop(message: &str) -> bool {
    message
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.eq_ignore_ascii_case("AUTO_MODE_NEXT=stop"))
}

pub fn build_continue_prompt(objective: Option<&str>, last_response: &str) -> String {
    let mut prompt = String::from(
        "Use the most recent context and proceed without waiting for user input.\n\n\
If available, use $session-autopilot for the end-of-turn continuation policy. \
If that skill is not installed, follow the continuation policy in this prompt directly.\n\n",
    );

    if let Some(objective) = objective.filter(|value| !value.trim().is_empty()) {
        prompt.push_str("Session objective:\n");
        prompt.push_str(objective.trim());
        prompt.push_str("\n\n");
    }

    prompt.push_str("Priority:\n");
    prompt.push_str("1. If the most recent user message contains explicit tasks or questions, execute those first.\n");
    prompt.push_str("2. Otherwise identify remaining concrete tasks, newly discovered tasks, and any TODO or plan documents in the repo, then reweight them.\n");
    prompt.push_str("3. Execute the highest-leverage next task or a tight batch of related tasks that compound.\n\n");
    prompt.push_str("Execution style:\n");
    prompt.push_str("- Prefer fundamental fixes over ad-hoc tweaks.\n");
    prompt.push_str("- Keep documentation and implementation in sync.\n");
    prompt.push_str("- Avoid repeatedly partial work on the same theme across many turns; batch related cleanup or refactor work together and finish a meaningful slice before ending.\n");
    prompt.push_str("- If you start a cleanup or refactor batch, do not stop mid-batch while obvious adjacent removals or rewires remain and can be completed safely in the same turn.\n");
    prompt.push_str("- Run appropriate verification for the changes you make.\n");
    prompt.push_str("- If you change code or docs, show `git diff --stat`, commit, and push when the repo has a writable remote and pushing is permitted.\n");
    prompt.push_str("- Default to continuing unless the project goal is achieved and no concrete task remains.\n\n");
    prompt.push_str("Last assistant response from the immediately previous turn:\n");
    prompt.push_str("<<<LAST_RESPONSE\n");
    prompt.push_str(last_response.trim());
    prompt.push_str("\nLAST_RESPONSE>>>\n\n");
    prompt.push_str("End your final response with exactly one line:\n");
    prompt.push_str("AUTO_MODE_NEXT=continue\n");
    prompt.push_str("or\n");
    prompt.push_str("AUTO_MODE_NEXT=stop\n");
    prompt
}

#[cfg(test)]
mod tests {
    use super::build_continue_prompt;
    use super::parse_auto_mode_stop;

    #[test]
    fn detects_explicit_stop_marker() {
        assert!(parse_auto_mode_stop("done\nAUTO_MODE_NEXT=stop\n"));
        assert!(!parse_auto_mode_stop("done\nAUTO_MODE_NEXT=continue\n"));
        assert!(!parse_auto_mode_stop("done without marker"));
    }

    #[test]
    fn continue_prompt_includes_last_response_and_footer_contract() {
        let prompt = build_continue_prompt(Some("Ship the feature"), "Implemented part A.");
        assert!(prompt.contains("Ship the feature"));
        assert!(prompt.contains("Implemented part A."));
        assert!(prompt.contains("$session-autopilot"));
        assert!(prompt.contains("If that skill is not installed"));
        assert!(prompt.contains("Avoid repeatedly partial work on the same theme"));
        assert!(prompt.contains("do not stop mid-batch"));
        assert!(prompt.contains("AUTO_MODE_NEXT=continue"));
        assert!(prompt.contains("AUTO_MODE_NEXT=stop"));
    }
}
