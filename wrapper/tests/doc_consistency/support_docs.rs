use super::{assert_contains, assert_contains_case_insensitive, read_repo_file, repo_root};

#[test]
fn support_claim_source_docs_exist_and_are_linked() {
    let readme = read_repo_file("README.md");
    let todos = read_repo_file("TODOS.md");
    let broker_contract = read_repo_file("docs/codexw-broker-adapter-contract.md");
    let broker_status = read_repo_file("docs/codexw-broker-adapter-status.md");
    let broker_client_policy = read_repo_file("docs/codexw-broker-client-policy.md");
    let broker_out_of_scope = read_repo_file("docs/codexw-broker-out-of-scope.md");
    let broker_endpoint_audit = read_repo_file("docs/codexw-broker-endpoint-audit.md");
    let broker_proof = read_repo_file("docs/codexw-broker-proof-matrix.md");
    let broker_policy = read_repo_file("docs/codexw-broker-support-policy.md");
    let broker_hardening = read_repo_file("docs/codexw-broker-hardening-catalog.md");
    let broker_decision = read_repo_file("docs/codexw-broker-connector-decision.md");
    let broker_promotion = read_repo_file("docs/codexw-broker-adapter-promotion.md");
    let broker_mapping = read_repo_file("docs/codexw-broker-connector-mapping.md");
    let broker_compat_target = read_repo_file("docs/codexw-broker-compatibility-target.md");
    let broker_adapter_plan = read_repo_file("docs/codexw-broker-connector-adapter-plan.md");
    let broker_shared_assumptions = read_repo_file("docs/codexw-broker-shared-assumptions.md");
    let broker_session_identity = read_repo_file("docs/codexw-broker-session-identity.md");
    let native_boundaries = read_repo_file("docs/codexw-native-support-boundaries.md");
    let native_status = read_repo_file("docs/codexw-native-product-status.md");
    let native_proof = read_repo_file("docs/codexw-native-proof-matrix.md");
    let native_policy = read_repo_file("docs/codexw-native-support-policy.md");
    let native_hardening = read_repo_file("docs/codexw-native-hardening-catalog.md");
    let self_evolution = read_repo_file("docs/codexw-self-evolution.md");
    let self_evolution_plan = read_repo_file("docs/codexw-self-evolution-implementation-plan.md");
    let self_supervision = read_repo_file("docs/codexw-self-supervision.md");
    let self_supervision_plan =
        read_repo_file("docs/codexw-self-supervision-implementation-plan.md");
    let plugin_system = read_repo_file("docs/codexw-plugin-system.md");
    let plugin_system_plan = read_repo_file("docs/codexw-plugin-system-implementation-plan.md");
    let local_api_sketch = read_repo_file("docs/codexw-local-api-sketch.md");
    let local_api_plan = read_repo_file("docs/codexw-local-api-implementation-plan.md");
    let local_api_route_matrix = read_repo_file("docs/codexw-local-api-route-matrix.md");
    let broker_host_matrix = read_repo_file("docs/codexw-broker-host-examination-matrix.md");
    let cross_deployment = read_repo_file("docs/codexw-cross-deployment-collaboration.md");
    let cross_project_dependency =
        read_repo_file("docs/codexw-cross-project-dependency-collaboration.md");
    let cross_project_contract =
        read_repo_file("docs/codexw-cross-project-dependency-contract-sketch.md");
    let cross_project_plan =
        read_repo_file("docs/codexw-cross-project-dependency-implementation-plan.md");
    let cross_deployment_contract =
        read_repo_file("docs/codexw-cross-deployment-handoff-contract-sketch.md");
    let cross_deployment_plan =
        read_repo_file("docs/codexw-cross-deployment-handoff-implementation-plan.md");
    let broker_artifact_sketch = read_repo_file("docs/codexw-broker-artifact-contract-sketch.md");
    let broker_artifact_plan = read_repo_file("docs/codexw-broker-artifact-implementation-plan.md");
    let workspace_policy = read_repo_file("docs/codexw-workspace-tool-policy.md");
    let broker_client_arch = read_repo_file("docs/codexw-broker-client-architecture.md");
    let broker_client_fixture = read_repo_file("docs/codexw-broker-client-fixture.md");
    let broker_handoff = read_repo_file("docs/codexw-broker-integration-handoff.md");
    let checklist = read_repo_file("docs/codexw-support-claim-checklist.md");

    for file in [
        "docs/codexw-broker-adapter-contract.md",
        "docs/codexw-broker-adapter-status.md",
        "docs/codexw-broker-client-policy.md",
        "docs/codexw-broker-out-of-scope.md",
        "docs/codexw-broker-proof-matrix.md",
        "docs/codexw-broker-support-policy.md",
        "docs/codexw-broker-hardening-catalog.md",
        "docs/codexw-broker-connector-decision.md",
        "docs/codexw-broker-compatibility-target.md",
        "docs/codexw-broker-connector-adapter-plan.md",
        "docs/codexw-broker-shared-assumptions.md",
        "docs/codexw-broker-session-identity.md",
        "docs/codexw-native-support-boundaries.md",
        "docs/codexw-native-product-status.md",
        "docs/codexw-native-proof-matrix.md",
        "docs/codexw-native-support-policy.md",
        "docs/codexw-native-hardening-catalog.md",
        "docs/codexw-self-evolution.md",
        "docs/codexw-self-evolution-implementation-plan.md",
        "docs/codexw-self-supervision.md",
        "docs/codexw-self-supervision-implementation-plan.md",
        "docs/codexw-plugin-system.md",
        "docs/codexw-plugin-system-implementation-plan.md",
        "docs/codexw-broker-client-architecture.md",
        "docs/codexw-cross-deployment-collaboration.md",
        "docs/codexw-cross-project-dependency-collaboration.md",
        "docs/codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-cross-project-dependency-implementation-plan.md",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
        "docs/codexw-broker-client-fixture.md",
        "docs/codexw-broker-host-examination-matrix.md",
        "docs/codexw-broker-integration-handoff.md",
        "docs/codexw-broker-artifact-contract-sketch.md",
        "docs/codexw-broker-artifact-implementation-plan.md",
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
        "docs/codexw-broker-client-architecture.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-deployment-collaboration.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-project-dependency-collaboration.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-project-dependency-contract-sketch.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-project-dependency-implementation-plan.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-broker-host-examination-matrix.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-broker-integration-handoff.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-broker-artifact-contract-sketch.md",
        "README.md",
    );
    assert_contains(
        &readme,
        "docs/codexw-broker-artifact-implementation-plan.md",
        "README.md",
    );
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
    assert_contains(&readme, "docs/codexw-self-evolution.md", "README.md");
    assert_contains(
        &readme,
        "docs/codexw-self-evolution-implementation-plan.md",
        "README.md",
    );
    assert_contains(&readme, "docs/codexw-self-supervision.md", "README.md");
    assert_contains(
        &readme,
        "docs/codexw-self-supervision-implementation-plan.md",
        "README.md",
    );
    assert_contains(&readme, "docs/codexw-plugin-system.md", "README.md");
    assert_contains(
        &readme,
        "docs/codexw-plugin-system-implementation-plan.md",
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
        "docs/codexw-broker-client-architecture.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-deployment-collaboration.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-project-dependency-collaboration.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-project-dependency-contract-sketch.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-project-dependency-implementation-plan.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-broker-host-examination-matrix.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-broker-integration-handoff.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-broker-artifact-contract-sketch.md",
        "TODOS.md",
    );
    assert_contains(
        &todos,
        "docs/codexw-broker-artifact-implementation-plan.md",
        "TODOS.md",
    );
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
    assert_contains(&todos, "docs/codexw-self-evolution.md", "TODOS.md");
    assert_contains(
        &todos,
        "docs/codexw-self-evolution-implementation-plan.md",
        "TODOS.md",
    );
    assert_contains(&todos, "docs/codexw-self-supervision.md", "TODOS.md");
    assert_contains(
        &todos,
        "docs/codexw-self-supervision-implementation-plan.md",
        "TODOS.md",
    );
    assert_contains(&todos, "docs/codexw-plugin-system.md", "TODOS.md");
    assert_contains(
        &todos,
        "docs/codexw-plugin-system-implementation-plan.md",
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
        &broker_status,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &broker_status,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-broker-adapter-status.md",
    );
    assert_contains(
        &broker_proof,
        "supported experimental adapter",
        "docs/codexw-broker-proof-matrix.md",
    );
    assert_contains(
        &broker_proof,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-broker-proof-matrix.md",
    );
    assert_contains(
        &broker_proof,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-broker-proof-matrix.md",
    );
    assert_contains(
        &broker_policy,
        "supported experimental adapter",
        "docs/codexw-broker-support-policy.md",
    );
    assert_contains_case_insensitive(
        &broker_policy,
        "project-assignment or dependency-edge surface",
        "docs/codexw-broker-support-policy.md",
    );
    assert_contains_case_insensitive(
        &broker_out_of_scope,
        "out of scope",
        "docs/codexw-broker-out-of-scope.md",
    );
    assert_contains_case_insensitive(
        &broker_out_of_scope,
        "project-assignment or dependency-edge contract",
        "docs/codexw-broker-out-of-scope.md",
    );
    assert_contains(
        &broker_endpoint_audit,
        "codexw-broker-artifact-implementation-plan.md",
        "docs/codexw-broker-endpoint-audit.md",
    );
    assert_contains(
        &broker_endpoint_audit,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-broker-endpoint-audit.md",
    );
    assert_contains(
        &broker_endpoint_audit,
        "session-scoped artifact index/detail/content",
        "docs/codexw-broker-endpoint-audit.md",
    );
    assert_contains(
        &broker_endpoint_audit,
        "session-scoped project assignment",
        "docs/codexw-broker-endpoint-audit.md",
    );
    assert_contains(
        &broker_endpoint_audit,
        "project dependency edges",
        "docs/codexw-broker-endpoint-audit.md",
    );
    assert_contains(
        &broker_hardening,
        "not a blocker",
        "docs/codexw-broker-hardening-catalog.md",
    );
    assert_contains_case_insensitive(
        &broker_hardening,
        "artifact-centric consumer story",
        "docs/codexw-broker-hardening-catalog.md",
    );
    assert_contains_case_insensitive(
        &broker_decision,
        "broker-backed clients such as app and webui",
        "docs/codexw-broker-connector-decision.md",
    );
    assert_contains_case_insensitive(
        &broker_decision,
        "artifact index/detail/content",
        "docs/codexw-broker-connector-decision.md",
    );
    assert_contains(
        &broker_promotion,
        "codexw-broker-artifact-contract-sketch.md",
        "docs/codexw-broker-adapter-promotion.md",
    );
    assert_contains_case_insensitive(
        &broker_promotion,
        "artifact index/detail/content routes",
        "docs/codexw-broker-adapter-promotion.md",
    );
    assert_contains(
        &broker_promotion,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-broker-adapter-promotion.md",
    );
    assert_contains_case_insensitive(
        &broker_promotion,
        "project-assignment and dependency-edge routes",
        "docs/codexw-broker-adapter-promotion.md",
    );
    assert_contains(
        &broker_mapping,
        "codexw-broker-artifact-implementation-plan.md",
        "docs/codexw-broker-connector-mapping.md",
    );
    assert_contains_case_insensitive(
        &broker_mapping,
        "any artifact index/detail/content route",
        "docs/codexw-broker-connector-mapping.md",
    );
    assert_contains_case_insensitive(
        &broker_mapping,
        "project-assignment or dependency-edge route",
        "docs/codexw-broker-connector-mapping.md",
    );
    assert_contains_case_insensitive(
        &broker_compat_target,
        "app/webui",
        "docs/codexw-broker-compatibility-target.md",
    );
    assert_contains_case_insensitive(
        &broker_compat_target,
        "host shell examination",
        "docs/codexw-broker-compatibility-target.md",
    );
    assert_contains(
        &broker_compat_target,
        "codexw-broker-artifact-contract-sketch.md",
        "docs/codexw-broker-compatibility-target.md",
    );
    assert_contains_case_insensitive(
        &broker_adapter_plan,
        "host examination shell-first",
        "docs/codexw-broker-connector-adapter-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_adapter_plan,
        "artifact index/detail/content",
        "docs/codexw-broker-connector-adapter-plan.md",
    );
    assert_contains(
        &broker_adapter_plan,
        "codexw-broker-artifact-implementation-plan.md",
        "docs/codexw-broker-connector-adapter-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_shared_assumptions,
        "broker-backed app/webui clients",
        "docs/codexw-broker-shared-assumptions.md",
    );
    assert_contains_case_insensitive(
        &broker_shared_assumptions,
        "dedicated artifact catalog",
        "docs/codexw-broker-shared-assumptions.md",
    );
    assert_contains_case_insensitive(
        &broker_session_identity,
        "broker-backed app/webui clients",
        "docs/codexw-broker-session-identity.md",
    );
    assert_contains_case_insensitive(
        &broker_session_identity,
        "artifact entries",
        "docs/codexw-broker-session-identity.md",
    );
    assert_contains_case_insensitive(
        &broker_client_policy,
        "shell-first host-examination posture",
        "docs/codexw-broker-client-policy.md",
    );
    assert_contains_case_insensitive(
        &broker_client_policy,
        "artifact surface exists",
        "docs/codexw-broker-client-policy.md",
    );
    assert_contains_case_insensitive(
        &broker_client_policy,
        "project-assignment or dependency-edge collaboration surface exists",
        "docs/codexw-broker-client-policy.md",
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
        &self_evolution,
        "safely hands off to a newer binary",
        "docs/codexw-self-evolution.md",
    );
    assert_contains_case_insensitive(
        &self_evolution,
        "checkpoint",
        "docs/codexw-self-evolution.md",
    );
    assert_contains_case_insensitive(
        &self_evolution,
        "standalone local-runtime-first",
        "docs/codexw-self-evolution.md",
    );
    assert_contains_case_insensitive(
        &self_evolution,
        "plugin-aware",
        "docs/codexw-self-evolution.md",
    );
    assert_contains_case_insensitive(
        &self_evolution_plan,
        "resume-handoff",
        "docs/codexw-self-evolution-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &self_evolution_plan,
        "old process does not exit before acknowledgment",
        "docs/codexw-self-evolution-implementation-plan.md",
    );
    assert_contains_case_insensitive(&self_evolution, "git repo", "docs/codexw-self-evolution.md");
    assert_contains_case_insensitive(
        &self_evolution_plan,
        "git pull",
        "docs/codexw-self-evolution-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &self_evolution_plan,
        "plugin versus core decision policy",
        "docs/codexw-self-evolution-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &self_supervision,
        "no `codexw` tool use or shell exec should be allowed to hang the runtime indefinitely",
        "docs/codexw-self-supervision.md",
    );
    assert_contains_case_insensitive(
        &self_supervision,
        "standalone local-runtime-first",
        "docs/codexw-self-supervision.md",
    );
    assert_contains_case_insensitive(
        &self_supervision,
        "background-shell dynamic tools must not execute in a way that freezes the input loop indefinitely",
        "docs/codexw-self-supervision.md",
    );
    assert_contains_case_insensitive(
        &self_supervision_plan,
        "background-shell dynamic tools",
        "docs/codexw-self-supervision-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &self_supervision_plan,
        "plugin-first",
        "docs/codexw-self-supervision-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &plugin_system,
        "voice reminder over the host speaker",
        "docs/codexw-plugin-system.md",
    );
    assert_contains_case_insensitive(
        &plugin_system,
        "https://github.com/lzbgt/codexw-plugins",
        "docs/codexw-plugin-system.md",
    );
    assert_contains_case_insensitive(
        &plugin_system,
        "self-evolution",
        "docs/codexw-plugin-system.md",
    );
    assert_contains_case_insensitive(
        &plugin_system_plan,
        "trusted-source checks",
        "docs/codexw-plugin-system-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &plugin_system_plan,
        "plugin install/update over full core replacement",
        "docs/codexw-plugin-system-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_client_arch,
        "app/webui",
        "docs/codexw-broker-client-architecture.md",
    );
    assert_contains_case_insensitive(
        &broker_client_arch,
        "host shell",
        "docs/codexw-broker-client-architecture.md",
    );
    assert_contains_case_insensitive(
        &broker_client_arch,
        "workspace dynamic tools",
        "docs/codexw-broker-client-architecture.md",
    );
    assert_contains_case_insensitive(
        &broker_client_arch,
        "cross-deployment",
        "docs/codexw-broker-client-architecture.md",
    );
    assert_contains_case_insensitive(
        &broker_host_matrix,
        "artifact catalog",
        "docs/codexw-broker-host-examination-matrix.md",
    );
    assert_contains_case_insensitive(
        &broker_host_matrix,
        "shell-first",
        "docs/codexw-broker-host-examination-matrix.md",
    );
    assert_contains_case_insensitive(
        &broker_host_matrix,
        "usable with caveat",
        "docs/codexw-broker-host-examination-matrix.md",
    );
    assert_contains_case_insensitive(
        &broker_handoff,
        "sibling `~/work/agent` workspace",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains_case_insensitive(
        &broker_handoff,
        "shell-first",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains_case_insensitive(
        &broker_handoff,
        "artifact list/detail/content api",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-broker-artifact-implementation-plan.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-deployment-collaboration.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-project-dependency-collaboration.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-project-dependency-implementation-plan.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-deployment-handoff-contract-sketch.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains(
        &broker_handoff,
        "codexw-cross-deployment-handoff-implementation-plan.md",
        "docs/codexw-broker-integration-handoff.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment,
        "work handoff",
        "docs/codexw-cross-deployment-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment,
        "session-scoped",
        "docs/codexw-cross-deployment-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment,
        "broker-mediated",
        "docs/codexw-cross-deployment-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment,
        "may not coexist on one host",
        "docs/codexw-cross-deployment-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment,
        "artifact-contract gap",
        "docs/codexw-cross-deployment-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_project_dependency,
        "projects may depend on one another",
        "docs/codexw-cross-project-dependency-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_project_dependency,
        "broker-mediated",
        "docs/codexw-cross-project-dependency-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_project_dependency,
        "dependency edge",
        "docs/codexw-cross-project-dependency-collaboration.md",
    );
    assert_contains_case_insensitive(
        &cross_project_contract,
        "session_project_bound",
        "docs/codexw-cross-project-dependency-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_project_contract,
        "project_dependency_declared",
        "docs/codexw-cross-project-dependency-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_project_contract,
        "session-to-project assignment",
        "docs/codexw-cross-project-dependency-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_project_plan,
        "dependency-edge create/list/detail routes",
        "docs/codexw-cross-project-dependency-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_project_plan,
        "handoff creation can reference dependency ids",
        "docs/codexw-cross-project-dependency-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_project_plan,
        "do not assume the participating deployments share a host",
        "docs/codexw-cross-project-dependency-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_contract,
        "handoff record",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_contract,
        "source.project_id",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_contract,
        "dependency_refs",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_contract,
        "session_handoff_proposed",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_contract,
        "artifact api",
        "docs/codexw-cross-deployment-handoff-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_plan,
        "accept/decline/complete",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_plan,
        "target project identity",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_plan,
        "same-host",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_plan,
        "route family exists",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &cross_deployment_plan,
        "do not fake handoff",
        "docs/codexw-cross-deployment-handoff-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_client_fixture,
        "shell-first host examination",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains_case_insensitive(
        &broker_client_fixture,
        "artifact index/detail/content route family",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains(
        &broker_client_fixture,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains(
        &broker_client_fixture,
        "codexw-broker-artifact-implementation-plan.md",
        "docs/codexw-broker-client-fixture.md",
    );
    assert_contains(
        &broker_contract,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-broker-adapter-contract.md",
    );
    assert_contains(
        &broker_contract,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-broker-adapter-contract.md",
    );
    assert_contains(
        &local_api_sketch,
        "docs/codexw-broker-integration-handoff.md",
        "docs/codexw-local-api-sketch.md",
    );
    assert_contains_case_insensitive(
        &local_api_sketch,
        "shell-first",
        "docs/codexw-local-api-sketch.md",
    );
    assert_contains(
        &local_api_plan,
        "docs/codexw-broker-integration-handoff.md",
        "docs/codexw-local-api-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &local_api_plan,
        "shell-first",
        "docs/codexw-local-api-implementation-plan.md",
    );
    assert_contains(
        &local_api_route_matrix,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-local-api-route-matrix.md",
    );
    assert_contains_case_insensitive(
        &local_api_route_matrix,
        "artifact list/detail/content api",
        "docs/codexw-local-api-route-matrix.md",
    );
    assert_contains_case_insensitive(
        &local_api_route_matrix,
        "future project/dependency collaboration track",
        "docs/codexw-local-api-route-matrix.md",
    );
    assert_contains(
        &local_api_route_matrix,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-local-api-route-matrix.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_sketch,
        "artifact index",
        "docs/codexw-broker-artifact-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_sketch,
        "artifact detail",
        "docs/codexw-broker-artifact-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_sketch,
        "shell-first",
        "docs/codexw-broker-artifact-contract-sketch.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_plan,
        "artifact index route",
        "docs/codexw-broker-artifact-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_plan,
        "artifact detail route",
        "docs/codexw-broker-artifact-implementation-plan.md",
    );
    assert_contains_case_insensitive(
        &broker_artifact_plan,
        "do not implement the content route",
        "docs/codexw-broker-artifact-implementation-plan.md",
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
        "codexw-broker-host-examination-matrix.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-integration-handoff.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-client-architecture.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-compatibility-target.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-connector-decision.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-connector-mapping.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-session-identity.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-client-policy.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-artifact-contract-sketch.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-cross-project-dependency-contract-sketch.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-cross-project-dependency-implementation-plan.md",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains(
        &checklist,
        "codexw-broker-artifact-implementation-plan.md",
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
    assert_contains_case_insensitive(
        &checklist,
        "broker-backed app/webui clients",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains_case_insensitive(
        &checklist,
        "fixture docs still describe shell-first host examination",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains_case_insensitive(
        &checklist,
        "project-assignment and dependency-edge route family",
        "docs/codexw-support-claim-checklist.md",
    );
    assert_contains_case_insensitive(
        &checklist,
        "sibling `~/work/agent` workspace",
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
        "legacy workspace compatibility failure",
        "docs/codexw-workspace-tool-policy.md",
    );
    assert_contains(
        &workspace_policy,
        "transcript and history summaries",
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

    assert_contains_case_insensitive(
        &broker_recommendation,
        "project-assignment or dependency-edge route family",
        "docs/codexw-broker-promotion-recommendation.md",
    );

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
