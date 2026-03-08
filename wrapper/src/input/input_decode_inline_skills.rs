pub(crate) fn mention_skill_path(path: &str) -> Option<String> {
    if let Some(stripped) = path.strip_prefix("skill://")
        && !stripped.is_empty()
    {
        return Some(stripped.to_string());
    }
    if path
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
    {
        return Some(path.to_string());
    }
    None
}
