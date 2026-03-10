use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("wrapper crate should live under repo root")
        .to_path_buf()
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", path.display(), err))
}

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "expected `{}` to contain `{}`",
        context,
        needle
    );
}

fn assert_contains_case_insensitive(haystack: &str, needle: &str, context: &str) {
    let haystack_lower = haystack.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    assert!(
        haystack_lower.contains(&needle_lower),
        "expected `{}` to contain `{}` (case-insensitive)",
        context,
        needle
    );
}

fn assert_not_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        !haystack.contains(needle),
        "expected `{}` to not contain `{}`",
        context,
        needle
    );
}

#[test]
fn support_claim_source_docs_exist_and_are_linked() {
    let readme = read_repo_file("README.md");
    let todos = read_repo_file("TODOS.md");
    let broker_contract = read_repo_file("docs/codexw-broker-adapter-contract.md");
    let broker_status = read_repo_file("docs/codexw-broker-adapter-status.md");
    let _broker_client_policy = read_repo_file("docs/codexw-broker-client-policy.md");
    let broker_out_of_scope = read_repo_file("docs/codexw-broker-out-of-scope.md");
    let broker_proof = read_repo_file("docs/codexw-broker-proof-matrix.md");
    let broker_policy = read_repo_file("docs/codexw-broker-support-policy.md");
    let broker_hardening = read_repo_file("docs/codexw-broker-hardening-catalog.md");
    let native_boundaries = read_repo_file("docs/codexw-native-support-boundaries.md");
    let native_status = read_repo_file("docs/codexw-native-product-status.md");
    let native_proof = read_repo_file("docs/codexw-native-proof-matrix.md");
    let native_policy = read_repo_file("docs/codexw-native-support-policy.md");
    let native_hardening = read_repo_file("docs/codexw-native-hardening-catalog.md");
    let checklist = read_repo_file("docs/codexw-support-claim-checklist.md");

    for file in [
        "docs/codexw-broker-adapter-contract.md",
        "docs/codexw-broker-adapter-status.md",
        "docs/codexw-broker-client-policy.md",
        "docs/codexw-broker-out-of-scope.md",
        "docs/codexw-broker-proof-matrix.md",
        "docs/codexw-broker-support-policy.md",
        "docs/codexw-broker-hardening-catalog.md",
        "docs/codexw-native-support-boundaries.md",
        "docs/codexw-native-product-status.md",
        "docs/codexw-native-proof-matrix.md",
        "docs/codexw-native-support-policy.md",
        "docs/codexw-native-hardening-catalog.md",
        "docs/codexw-support-claim-checklist.md",
    ] {
        let path = repo_root().join(file);
        assert!(path.exists(), "expected {} to exist", path.display());
    }

    assert_contains(
        &readme,
        "docs/codexw-broker-adapter-contract.md",
        "README.md",
    );
    assert_contains(&readme, "docs/codexw-broker-adapter-status.md", "README.md");
    assert_contains(&readme, "docs/codexw-broker-client-policy.md", "README.md");
    assert_contains(&readme, "docs/codexw-broker-out-of-scope.md", "README.md");
    assert_contains(&readme, "docs/codexw-broker-proof-matrix.md", "README.md");
    assert_contains(
        &readme,
        "docs/codexw-broker-hardening-catalog.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-native-support-boundaries.md",
        "README.md",
    );
    assert_contains(&readme, "docs/codexw-native-product-status.md", "README.md");
    assert_contains(&readme, "docs/codexw-native-proof-matrix.md", "README.md");
    assert_contains(
        &readme,
        "docs/codexw-native-hardening-catalog.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-support-claim-checklist.md",
        "README.md",
    );

    assert_contains(&todos, "docs/codexw-broker-adapter-contract.md", "TODOS.md");
    assert_contains(&todos, "docs/codexw-broker-adapter-status.md", "TODOS.md");
    assert_contains(&todos, "docs/codexw-broker-client-policy.md", "TODOS.md");
    assert_contains(&todos, "docs/codexw-broker-out-of-scope.md", "TODOS.md");
    assert_contains(&todos, "docs/codexw-broker-proof-matrix.md", "TODOS.md");
    assert_contains(
        &todos,
        "docs/codexw-broker-hardening-catalog.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-native-support-boundaries.md",
        "TODOS.md",
    );
    assert_contains(&todos, "docs/codexw-native-product-status.md", "TODOS.md");
    assert_contains(&todos, "docs/codexw-native-proof-matrix.md", "TODOS.md");
    assert_contains(
        &todos,
        "docs/codexw-native-hardening-catalog.md",
        "TODOS.md",
    );
    assert_contains(&todos, "docs/codexw-support-claim-checklist.md", "TODOS.md");

    assert_contains(
        &broker_contract,
        "owner",
        "docs/codexw-broker-adapter-contract.md",
    );
    assert_contains(
        &broker_contract,
        "observer",
        "docs/codexw-broker-adapter-contract.md",
    );
    assert_contains(
        &broker_contract,
        "rival",
        "docs/codexw-broker-adapter-contract.md",
    );
    assert_contains(
        &broker_status,
        "supported experimental adapter",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &broker_proof,
        "supported experimental adapter",
        "docs/codexw-broker-proof-matrix.md",
    );
    assert_contains(
        &broker_policy,
        "supported experimental adapter",
        "docs/codexw-broker-support-policy.md",
    );
    assert_contains_case_insensitive(
        &broker_out_of_scope,
        "out of scope",
        "docs/codexw-broker-out-of-scope.md",
    );
    assert_contains(
        &broker_hardening,
        "not a blocker",
        "docs/codexw-broker-hardening-catalog.md",
    );

    assert_contains_case_insensitive(
        &native_boundaries,
        "alternate-screen",
        "docs/codexw-native-support-boundaries.md",
    );
    assert_contains_case_insensitive(
        &native_boundaries,
        "audio",
        "docs/codexw-native-support-boundaries.md",
    );
    assert_contains_case_insensitive(
        &native_status,
        "terminal-first",
        "docs/codexw-native-product-status.md",
    );
    assert_contains_case_insensitive(
        &native_status,
        "scrollback-first",
        "docs/codexw-native-product-status.md",
    );
    assert_contains_case_insensitive(
        &native_proof,
        "terminal-first",
        "docs/codexw-native-proof-matrix.md",
    );
    assert_contains_case_insensitive(
        &native_proof,
        "scrollback-first",
        "docs/codexw-native-proof-matrix.md",
    );
    assert_contains_case_insensitive(
        &native_policy,
        "terminal-first",
        "docs/codexw-native-support-policy.md",
    );
    assert_contains_case_insensitive(
        &native_policy,
        "scrollback-first",
        "docs/codexw-native-support-policy.md",
    );
    assert_contains_case_insensitive(
        &native_hardening,
        "not currently a blocker",
        "docs/codexw-native-hardening-catalog.md",
    );

    assert_contains(
        &checklist,
        "codexw-broker-adapter-contract.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-adapter-status.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-out-of-scope.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-proof-matrix.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-hardening-catalog.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-native-support-boundaries.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-native-product-status.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-native-proof-matrix.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-native-hardening-catalog.md",
        "docs/codexw-support-claim-checklist.md",
    );
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
fn broker_and_native_docs_link_to_automated_support_claim_guard() {
    let checklist = read_repo_file("docs/codexw-support-claim-checklist.md");
    let broker_proof = read_repo_file("docs/codexw-broker-proof-matrix.md");
    let native_proof = read_repo_file("docs/codexw-native-proof-matrix.md");

    for contents in [&checklist, &broker_proof, &native_proof] {
        assert_contains(contents, "wrapper/tests/doc_consistency.rs", "doc text");
    }
}

#[test]
fn broker_docs_preserve_fixture_diversity_claims() {
    let broker_status = read_repo_file("docs/codexw-broker-adapter-status.md");
    let broker_fixture = read_repo_file("docs/codexw-broker-client-fixture.md");
    let broker_hardening = read_repo_file("docs/codexw-broker-hardening-catalog.md");
    let checklist = read_repo_file("docs/codexw-support-claim-checklist.md");

    assert_contains(
        &broker_status,
        "Python and Node",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &broker_status,
        "scripts/codexw_broker_client.py",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &broker_status,
        "scripts/codexw_broker_client_node.mjs",
        "docs/codexw-broker-adapter-status.md",
    );

    assert_contains(
        &broker_fixture,
        "scripts/codexw_broker_client.py",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains(
        &broker_fixture,
        "scripts/codexw_broker_client_node.mjs",
        "docs/codexw-broker-client-fixture.md",
    );

    assert_contains(
        &broker_hardening,
        "Python and Node fixtures",
        "docs/codexw-broker-hardening-catalog.md",
    );
    assert_contains(
        &checklist,
        "Python and Node",
        "docs/codexw-support-claim-checklist.md",
    );
}

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

#[test]
fn todos_keeps_active_work_separate_from_optional_hardening() {
    let todos = read_repo_file("TODOS.md");

    assert_contains(&todos, "## Highest-Leverage Active Work", "TODOS.md");
    assert_contains(&todos, "## Secondary Work", "TODOS.md");
    assert_contains(
        &todos,
        "### 4. Optional Broker Hardening Catalog Maintenance",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-broker-hardening-catalog.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-native-hardening-catalog.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "instead of treating them as active blockers by default",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "move an item from the hardening catalog back into this backlog only if:",
        "TODOS.md",
    );
}

#[test]
fn broker_and_native_support_docs_keep_current_support_level_wording() {
    let broker_status = read_repo_file("docs/codexw-broker-adapter-status.md");
    let broker_promotion = read_repo_file("docs/codexw-broker-adapter-promotion.md");
    let broker_recommendation = read_repo_file("docs/codexw-broker-promotion-recommendation.md");
    let broker_policy = read_repo_file("docs/codexw-broker-support-policy.md");
    let native_status = read_repo_file("docs/codexw-native-product-status.md");
    let native_recommendation = read_repo_file("docs/codexw-native-product-recommendation.md");
    let native_policy = read_repo_file("docs/codexw-native-support-policy.md");

    for (name, contents) in [
        ("docs/codexw-broker-adapter-status.md", &broker_status),
        ("docs/codexw-broker-adapter-promotion.md", &broker_promotion),
        (
            "docs/codexw-broker-promotion-recommendation.md",
            &broker_recommendation,
        ),
        ("docs/codexw-broker-support-policy.md", &broker_policy),
    ] {
        assert_contains(contents, "supported experimental adapter", name);
    }

    for (name, contents) in [
        ("docs/codexw-native-product-status.md", &native_status),
        (
            "docs/codexw-native-product-recommendation.md",
            &native_recommendation,
        ),
        ("docs/codexw-native-support-policy.md", &native_policy),
    ] {
        assert_contains_case_insensitive(contents, "terminal-first", name);
        assert_contains_case_insensitive(contents, "scrollback-first", name);
    }
}
