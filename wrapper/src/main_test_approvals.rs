use crate::events::params_auto_approval_result;
use crate::policy::choose_command_approval_decision;
use serde_json::json;

#[test]
fn yolo_prefers_first_available_command_approval_decision() {
    let params = json!({
        "availableDecisions": [
            "acceptForSession",
            "accept"
        ]
    });
    assert_eq!(
        choose_command_approval_decision(&params, true),
        json!("acceptForSession")
    );
}

#[test]
fn command_approval_defaults_to_accept() {
    assert_eq!(
        choose_command_approval_decision(&json!({}), false),
        json!("accept")
    );
}

#[test]
fn approval_prefers_allow_decisions_over_first_entry() {
    let params = json!({
        "availableDecisions": [
            "decline",
            "accept",
            "cancel"
        ]
    });
    assert_eq!(
        choose_command_approval_decision(&params, false),
        json!("accept")
    );
}

#[test]
fn generic_approval_prefers_session_accept_when_available() {
    let params = json!({
        "availableDecisions": [
            "decline",
            "acceptForSession",
            "accept"
        ]
    });
    assert_eq!(
        params_auto_approval_result(&params),
        json!({"decision": "acceptForSession"})
    );
}

#[test]
fn approval_prefers_network_allow_amendment_over_session_accept() {
    let params = json!({
        "availableDecisions": [
            "acceptForSession",
            {
                "applyNetworkPolicyAmendment": {
                    "networkPolicyAmendment": {
                        "host": "api.github.com",
                        "action": "allow"
                    }
                }
            },
            "accept"
        ]
    });
    assert_eq!(
        choose_command_approval_decision(&params, false),
        json!({
            "applyNetworkPolicyAmendment": {
                "networkPolicyAmendment": {
                    "host": "api.github.com",
                    "action": "allow"
                }
            }
        })
    );
}

#[test]
fn approval_prefers_execpolicy_amendment_over_plain_accept() {
    let params = json!({
        "availableDecisions": [
            "accept",
            {
                "acceptWithExecpolicyAmendment": {
                    "execpolicy_amendment": ["git", "status"]
                }
            }
        ]
    });
    assert_eq!(
        choose_command_approval_decision(&params, false),
        json!({
            "acceptWithExecpolicyAmendment": {
                "execpolicy_amendment": ["git", "status"]
            }
        })
    );
}
