use super::*;

#[path = "main_test_session_status_ps_orchestration/runtime.rs"]
mod runtime;

#[test]
fn ps_filter_parser_accepts_worker_class_aliases() {
    assert_eq!(parse_ps_filter(None), Some(WorkerFilter::All));
    assert_eq!(parse_ps_filter(Some("all")), Some(WorkerFilter::All));
    assert_eq!(
        parse_ps_filter(Some("guidance")),
        Some(WorkerFilter::Guidance)
    );
    assert_eq!(parse_ps_filter(Some("next")), Some(WorkerFilter::Guidance));
    assert_eq!(
        parse_ps_filter(Some("blockers")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(
        parse_ps_filter(Some("dependencies")),
        Some(WorkerFilter::Dependencies)
    );
    assert_eq!(
        parse_ps_filter(Some("deps")),
        Some(WorkerFilter::Dependencies)
    );
    assert_eq!(
        parse_ps_filter(Some("blocking")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(
        parse_ps_filter(Some("prereqs")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(parse_ps_filter(Some("agents")), Some(WorkerFilter::Agents));
    assert_eq!(parse_ps_filter(Some("shells")), Some(WorkerFilter::Shells));
    assert_eq!(
        parse_ps_filter(Some("services")),
        Some(WorkerFilter::Services)
    );
    assert_eq!(
        parse_ps_filter(Some("capabilities")),
        Some(WorkerFilter::Capabilities)
    );
    assert_eq!(
        parse_ps_filter(Some("caps")),
        Some(WorkerFilter::Capabilities)
    );
    assert_eq!(
        parse_ps_filter(Some("terminals")),
        Some(WorkerFilter::Terminals)
    );
    assert_eq!(
        parse_ps_filter(Some("actions")),
        Some(WorkerFilter::Actions)
    );
    assert_eq!(
        parse_ps_filter(Some("suggestions")),
        Some(WorkerFilter::Actions)
    );
    assert_eq!(parse_ps_filter(Some("clean")), None);
    assert_eq!(parse_ps_filter(Some("unknown")), None);
}

#[test]
fn ps_focus_capability_parser_accepts_capability_selector() {
    assert_eq!(
        parse_ps_focus_capability(&["@api.http"], ":ps actions").expect("parse capability"),
        "api.http"
    );
    assert!(parse_ps_focus_capability(&["api.http"], ":ps actions").is_err());
    assert!(parse_ps_focus_capability(&["@api.http", "@db.redis"], ":ps actions").is_err());
}

#[test]
fn ps_dependency_filter_parser_accepts_dependency_issue_aliases() {
    use crate::orchestration_view::DependencyFilter;

    assert_eq!(
        parse_ps_dependency_filter(None),
        Some(DependencyFilter::All)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("blocking")),
        Some(DependencyFilter::Blocking)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("blockers")),
        Some(DependencyFilter::Blocking)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("sidecars")),
        Some(DependencyFilter::Sidecars)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("missing")),
        Some(DependencyFilter::Missing)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("booting")),
        Some(DependencyFilter::Booting)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("ambiguous")),
        Some(DependencyFilter::Ambiguous)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("conflicts")),
        Some(DependencyFilter::Ambiguous)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("satisfied")),
        Some(DependencyFilter::Satisfied)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("ready")),
        Some(DependencyFilter::Satisfied)
    );
    assert_eq!(parse_ps_dependency_filter(Some("weird")), None);
}

#[test]
fn ps_dependency_selector_accepts_optional_capability_reference() {
    use crate::orchestration_view::DependencyFilter;
    use crate::orchestration_view::DependencySelection;

    assert_eq!(
        parse_ps_dependency_selector(&["missing", "@api.http"]).expect("selector"),
        DependencySelection {
            filter: DependencyFilter::Missing,
            capability: Some("api.http".to_string()),
        }
    );
    assert_eq!(
        parse_ps_dependency_selector(&["@api.http"]).expect("selector"),
        DependencySelection {
            filter: DependencyFilter::All,
            capability: Some("api.http".to_string()),
        }
    );
    assert!(parse_ps_dependency_selector(&["missing", "weird"]).is_err());
    assert!(parse_ps_dependency_selector(&["missing", "@api.http", "@frontend.dev"]).is_err());
}

#[test]
fn ps_capability_filter_parser_accepts_issue_aliases() {
    use crate::background_shells::BackgroundShellCapabilityIssueClass;

    assert_eq!(parse_ps_capability_issue_filter(None), Some(None));
    assert_eq!(parse_ps_capability_issue_filter(Some("all")), Some(None));
    assert_eq!(
        parse_ps_capability_issue_filter(Some("healthy")),
        Some(Some(BackgroundShellCapabilityIssueClass::Healthy))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("ok")),
        Some(Some(BackgroundShellCapabilityIssueClass::Healthy))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("missing")),
        Some(Some(BackgroundShellCapabilityIssueClass::Missing))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("booting")),
        Some(Some(BackgroundShellCapabilityIssueClass::Booting))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("ambiguous")),
        Some(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("conflicts")),
        Some(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("untracked")),
        Some(Some(BackgroundShellCapabilityIssueClass::Untracked))
    );
    assert_eq!(parse_ps_capability_issue_filter(Some("weird")), None);
}

#[test]
fn ps_service_selector_accepts_optional_capability_reference() {
    use crate::background_shells::BackgroundShellServiceIssueClass;

    assert_eq!(
        parse_ps_service_selector(&["ready", "@api.http"]).expect("selector"),
        (
            Some(BackgroundShellServiceIssueClass::Ready),
            Some("api.http".to_string()),
        )
    );
    assert_eq!(
        parse_ps_service_selector(&["@api.http"]).expect("selector"),
        (None, Some("api.http".to_string()))
    );
    assert!(parse_ps_service_selector(&["ready", "weird"]).is_err());
    assert!(parse_ps_service_selector(&["ready", "@bad!"]).is_err());
    assert!(parse_ps_service_selector(&["ready", "@api.http", "@frontend.dev"]).is_err());
}

#[test]
fn ps_service_filter_parser_accepts_issue_aliases() {
    use crate::background_shells::BackgroundShellServiceIssueClass;

    assert_eq!(parse_ps_service_issue_filter(None), Some(None));
    assert_eq!(parse_ps_service_issue_filter(Some("all")), Some(None));
    assert_eq!(
        parse_ps_service_issue_filter(Some("ready")),
        Some(Some(BackgroundShellServiceIssueClass::Ready))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("healthy")),
        Some(Some(BackgroundShellServiceIssueClass::Ready))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("booting")),
        Some(Some(BackgroundShellServiceIssueClass::Booting))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("untracked")),
        Some(Some(BackgroundShellServiceIssueClass::Untracked))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("conflicts")),
        Some(Some(BackgroundShellServiceIssueClass::Conflicts))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("ambiguous")),
        Some(Some(BackgroundShellServiceIssueClass::Conflicts))
    );
    assert_eq!(parse_ps_service_issue_filter(Some("weird")), None);
}

#[test]
fn clean_target_parser_accepts_scoped_cleanup_aliases() {
    use crate::dispatch_command_session_ps::CleanSelection;
    use crate::dispatch_command_session_ps::CleanTarget;

    assert_eq!(parse_clean_target(None), Some(CleanTarget::All));
    assert_eq!(parse_clean_target(Some("all")), Some(CleanTarget::All));
    assert_eq!(
        parse_clean_target(Some("blockers")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("blocking")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("prereqs")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("shells")),
        Some(CleanTarget::Shells)
    );
    assert_eq!(
        parse_clean_target(Some("services")),
        Some(CleanTarget::Services)
    );
    assert_eq!(
        parse_clean_target(Some("terminals")),
        Some(CleanTarget::Terminals)
    );
    assert_eq!(parse_clean_target(Some("agents")), None);
    assert_eq!(parse_clean_target(Some("unknown")), None);

    assert_eq!(
        parse_clean_selection(&["services", "@api.http"], ":ps clean").expect("clean selection"),
        CleanSelection {
            target: CleanTarget::Services,
            capability: Some("api.http".to_string()),
        }
    );
    assert_eq!(
        parse_clean_selection(&["blockers", "@api.http"], ":ps clean").expect("clean selection"),
        CleanSelection {
            target: CleanTarget::Blockers,
            capability: Some("api.http".to_string()),
        }
    );
    assert!(parse_clean_selection(&["shells", "@api.http"], ":ps clean").is_err());
}
