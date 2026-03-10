use super::{assert_contains, read_repo_file};

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
