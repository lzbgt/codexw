use std::fs;
use std::path::{Path, PathBuf};

#[path = "doc_consistency/backlog.rs"]
mod backlog;
#[path = "doc_consistency/framing.rs"]
mod framing;
#[path = "doc_consistency/naming.rs"]
mod naming;
#[path = "doc_consistency/support_docs.rs"]
mod support_docs;

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
