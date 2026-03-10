use super::{assert_contains, assert_not_contains, read_repo_file};

#[test]
fn current_state_docs_do_not_revert_to_connector_prototype_wording() {
    for relative in [
        "docs/codexw-broker-adapter-status.md",
        "docs/codexw-broker-client-fixture.md",
        "docs/codexw-broker-connectivity.md",
        "docs/codexw-local-api-event-sourcing.md",
        "docs/codexw-local-api-implementation-plan.md",
        "docs/codexw-broker-shared-assumptions.md",
    ] {
        let contents = read_repo_file(relative);
        assert_not_contains(&contents, "connector prototype", relative);
        assert_not_contains(&contents, "standalone connector prototype", relative);
        assert_not_contains(&contents, "current broker connector prototype", relative);
    }

    assert_contains(
        &read_repo_file("docs/codexw-broker-adapter-status.md"),
        "standalone connector adapter",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &read_repo_file("docs/codexw-broker-client-fixture.md"),
        "current connector adapter surface",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains(
        &read_repo_file("docs/codexw-local-api-implementation-plan.md"),
        "current broker connector adapter",
        "docs/codexw-local-api-implementation-plan.md",
    );
}

#[test]
fn current_state_docs_do_not_reintroduce_remaining_connector_wording_drifts() {
    let cases = [
        (
            "docs/codexw-broker-adapter-status.md",
            "prototype expansion",
        ),
        (
            "docs/codexw-broker-connectivity.md",
            "manual and prototype remote-control",
        ),
        (
            "docs/codexw-broker-connectivity.md",
            "first standalone prototype",
        ),
        (
            "docs/codexw-broker-connector-adapter-plan.md",
            "initial standalone prototype",
        ),
        (
            "docs/codexw-broker-connector-decision.md",
            "the connector prototype exists",
        ),
        (
            "docs/codexw-native-product-recommendation.md",
            "connector/prototype proof surface",
        ),
        (
            "docs/codexw-broker-client-fixture.md",
            "other prototype clients",
        ),
        (
            "docs/codexw-native-gap-assessment.md",
            "broker-style connector prototype",
        ),
    ];

    for (relative, stale_phrase) in cases {
        let contents = read_repo_file(relative);
        assert_not_contains(&contents, stale_phrase, relative);
    }
}

#[test]
fn support_claim_docs_do_not_reference_stale_broker_status_filename() {
    let docs = [
        ("README.md", read_repo_file("README.md")),
        ("TODOS.md", read_repo_file("TODOS.md")),
        (
            "docs/codexw-broker-adapter-contract.md",
            read_repo_file("docs/codexw-broker-adapter-contract.md"),
        ),
        (
            "docs/codexw-broker-adapter-promotion.md",
            read_repo_file("docs/codexw-broker-adapter-promotion.md"),
        ),
        (
            "docs/codexw-broker-client-policy.md",
            read_repo_file("docs/codexw-broker-client-policy.md"),
        ),
        (
            "docs/codexw-broker-connectivity.md",
            read_repo_file("docs/codexw-broker-connectivity.md"),
        ),
        (
            "docs/codexw-broker-out-of-scope.md",
            read_repo_file("docs/codexw-broker-out-of-scope.md"),
        ),
        (
            "docs/codexw-broker-proof-matrix.md",
            read_repo_file("docs/codexw-broker-proof-matrix.md"),
        ),
        (
            "docs/codexw-broker-support-policy.md",
            read_repo_file("docs/codexw-broker-support-policy.md"),
        ),
        (
            "docs/codexw-native-support-boundaries.md",
            read_repo_file("docs/codexw-native-support-boundaries.md"),
        ),
        (
            "docs/codexw-support-claim-checklist.md",
            read_repo_file("docs/codexw-support-claim-checklist.md"),
        ),
    ];

    for (name, contents) in docs {
        assert_not_contains(&contents, "codexw-broker-prototype-status.md", name);
    }
}

#[test]
fn current_state_docs_do_not_reference_stale_connector_plan_filename() {
    let docs = [
        ("README.md", read_repo_file("README.md")),
        ("TODOS.md", read_repo_file("TODOS.md")),
        (
            "docs/codexw-broker-adapter-status.md",
            read_repo_file("docs/codexw-broker-adapter-status.md"),
        ),
        (
            "docs/codexw-broker-adapter-promotion.md",
            read_repo_file("docs/codexw-broker-adapter-promotion.md"),
        ),
        (
            "docs/codexw-broker-client-fixture.md",
            read_repo_file("docs/codexw-broker-client-fixture.md"),
        ),
        (
            "docs/codexw-broker-connectivity.md",
            read_repo_file("docs/codexw-broker-connectivity.md"),
        ),
        (
            "docs/codexw-broker-proof-matrix.md",
            read_repo_file("docs/codexw-broker-proof-matrix.md"),
        ),
        (
            "docs/codexw-design.md",
            read_repo_file("docs/codexw-design.md"),
        ),
    ];

    for (name, contents) in docs {
        assert_not_contains(&contents, "codexw-broker-connector-prototype-plan.md", name);
    }
}

#[test]
fn current_state_broker_docs_do_not_regress_to_current_prototype_wording() {
    let docs = [
        (
            "docs/codexw-broker-adapter-promotion.md",
            read_repo_file("docs/codexw-broker-adapter-promotion.md"),
        ),
        (
            "docs/codexw-broker-connectivity.md",
            read_repo_file("docs/codexw-broker-connectivity.md"),
        ),
        (
            "docs/codexw-broker-out-of-scope.md",
            read_repo_file("docs/codexw-broker-out-of-scope.md"),
        ),
    ];

    for (name, contents) in docs {
        assert_not_contains(&contents, "current prototype", name);
        assert_not_contains(&contents, "prototype proof set", name);
        assert_not_contains(&contents, "prototype behavior note", name);
    }
}
