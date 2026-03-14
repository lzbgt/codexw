use super::{assert_contains, assert_contains_case_insensitive, read_repo_file, repo_root};

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
    let workspace_policy = read_repo_file("docs/codexw-workspace-tool-policy.md");
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
        "docs/codexw-workspace-tool-policy.md",
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
    assert_contains(&readme, "no longer advertised by default", "README.md");
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
    assert_contains(&todos, "docs/codexw-workspace-tool-policy.md", "TODOS.md");
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
    assert_contains_case_insensitive(
        &workspace_policy,
        "shell or python",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains_case_insensitive(
        &workspace_policy,
        "no longer advertise",
        "docs/codexw-workspace-tool-policy.md",
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
fn broker_and_native_docs_link_to_automated_support_claim_guard() {
    let checklist = read_repo_file("docs/codexw-support-claim-checklist.md");
    let broker_proof = read_repo_file("docs/codexw-broker-proof-matrix.md");
    let native_proof = read_repo_file("docs/codexw-native-proof-matrix.md");

    for contents in [&checklist, &broker_proof, &native_proof] {
        assert_contains(contents, "wrapper/tests/doc_consistency.rs", "doc text");
    }
}

#[test]
fn design_doc_keeps_workspace_tool_policy_note_linked() {
    let design = read_repo_file("docs/codexw-design.md");
    let workspace_policy = read_repo_file("docs/codexw-workspace-tool-policy.md");
    let readme = read_repo_file("README.md");

    assert_contains(
        &design,
        "codexw-workspace-tool-policy.md",
        "docs/codexw-design.md",
    );
    assert_contains(
        &workspace_policy,
        "workspace_read_file",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(
        &workspace_policy,
        "workspace_search_text",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(
        &workspace_policy,
        "shell is the general-purpose execution substrate",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(
        &workspace_policy,
        "bounded compatibility scan budget",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(
        &workspace_policy,
        "legacy workspace compatibility path",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(&readme, "legacy workspace compatibility path", "README.md");
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
