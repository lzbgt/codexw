#[path = "parse/args.rs"]
mod args;
#[path = "parse/selectors.rs"]
mod selectors;

pub(crate) use self::args::parse_operator_recipe_args;
pub(crate) use self::args::parse_optional_contract_field;
pub(crate) use self::args::parse_optional_contract_recipes;
pub(crate) use self::args::parse_ps_capability_list;
pub(crate) use self::args::parse_ps_contract_args;
pub(crate) use self::args::parse_ps_provide_capabilities;
pub(crate) use self::args::parse_ps_relabel_args;
pub(crate) use self::args::parse_ps_run_args;
pub(crate) use self::args::parse_ps_send_args;
pub(crate) use self::args::parse_ps_wait_timeout;
pub(crate) use self::selectors::parse_clean_selection;
#[cfg(test)]
pub(crate) use self::selectors::parse_clean_target;
pub(crate) use self::selectors::parse_ps_capability_issue_filter;
#[cfg(test)]
pub(crate) use self::selectors::parse_ps_dependency_filter;
pub(crate) use self::selectors::parse_ps_dependency_selector;
pub(crate) use self::selectors::parse_ps_filter;
pub(crate) use self::selectors::parse_ps_focus_capability;
#[cfg(test)]
pub(crate) use self::selectors::parse_ps_service_issue_filter;
pub(crate) use self::selectors::parse_ps_service_selector;

pub(super) fn is_valid_capability_ref(raw: &str) -> bool {
    raw.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
}
