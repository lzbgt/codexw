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
