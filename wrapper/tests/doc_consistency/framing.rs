use super::{assert_contains, assert_not_contains, read_repo_file};

#[test]
fn broker_docs_do_not_regress_to_stale_phase_wording_for_current_adapter_state() {
    let docs = [
        (
            "docs/codexw-broker-connector-decision.md",
            read_repo_file("docs/codexw-broker-connector-decision.md"),
        ),
        (
            "docs/codexw-broker-connectivity.md",
            read_repo_file("docs/codexw-broker-connectivity.md"),
        ),
        (
            "docs/codexw-local-api-implementation-plan.md",
            read_repo_file("docs/codexw-local-api-implementation-plan.md"),
        ),
        (
            "docs/codexw-local-api-route-matrix.md",
            read_repo_file("docs/codexw-local-api-route-matrix.md"),
        ),
    ];

    for (name, contents) in docs {
        assert_not_contains(&contents, "first phase", name);
        assert_not_contains(&contents, "first-phase", name);
        assert_not_contains(&contents, "Phase 0", name);
        assert_not_contains(&contents, "Phase 1", name);
        assert_not_contains(&contents, "Phase 2", name);
        assert_not_contains(&contents, "Phase 3", name);
        assert_not_contains(&contents, "Phase 4", name);
        assert_not_contains(&contents, "Phase 5", name);
        assert_not_contains(&contents, "phase 0", name);
        assert_not_contains(&contents, "phase 1", name);
        assert_not_contains(&contents, "phase 2", name);
        assert_not_contains(&contents, "phase 3", name);
        assert_not_contains(&contents, "phase 4", name);
        assert_not_contains(&contents, "phase 5", name);
    }
}

#[test]
fn broker_design_docs_do_not_regress_to_stale_remaining_question_wording() {
    let docs = [
        (
            "docs/codexw-broker-compatibility-target.md",
            read_repo_file("docs/codexw-broker-compatibility-target.md"),
        ),
        (
            "docs/codexw-broker-shared-assumptions.md",
            read_repo_file("docs/codexw-broker-shared-assumptions.md"),
        ),
        (
            "docs/codexw-broker-out-of-scope.md",
            read_repo_file("docs/codexw-broker-out-of-scope.md"),
        ),
    ];

    for (name, contents) in docs {
        assert_not_contains(&contents, "remaining design question", name);
        assert_not_contains(&contents, "remaining broker-design TODO", name);
    }
}

#[test]
fn historical_broker_and_local_api_docs_keep_their_record_framing() {
    let broker_connector_decision = read_repo_file("docs/codexw-broker-connector-decision.md");
    let local_api_sketch = read_repo_file("docs/codexw-local-api-sketch.md");
    let broker_compatibility_target = read_repo_file("docs/codexw-broker-compatibility-target.md");
    let broker_shared_assumptions = read_repo_file("docs/codexw-broker-shared-assumptions.md");
    let local_api_plan = read_repo_file("docs/codexw-local-api-implementation-plan.md");

    assert_contains(
        &broker_connector_decision,
        "historical decision record",
        "docs/codexw-broker-connector-decision.md",
    );
    assert_contains(
        &local_api_sketch,
        "conceptual companion",
        "docs/codexw-local-api-sketch.md",
    );
    assert_contains(
        &broker_compatibility_target,
        "records the broker compatibility target decision",
        "docs/codexw-broker-compatibility-target.md",
    );
    assert_contains(
        &broker_shared_assumptions,
        "records the broker shared-assumptions assessment",
        "docs/codexw-broker-shared-assumptions.md",
    );
    assert_contains(
        &local_api_plan,
        "Current Implementation Status",
        "docs/codexw-local-api-implementation-plan.md",
    );
}
