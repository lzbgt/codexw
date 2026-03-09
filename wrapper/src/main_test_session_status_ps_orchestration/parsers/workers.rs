use super::*;

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
