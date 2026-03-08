use serde_json::Value;

#[path = "catalog_file_search.rs"]
mod catalog_file_search;
#[path = "catalog_thread_list.rs"]
mod catalog_thread_list;

pub(crate) fn render_fuzzy_file_search_results(query: &str, files: &[Value]) -> String {
    catalog_file_search::render_fuzzy_file_search_results(query, files)
}

pub(crate) fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    catalog_thread_list::render_thread_list(result, search_term)
}

pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    catalog_thread_list::extract_thread_ids(result)
}

pub(crate) fn extract_file_search_paths(files: &[Value]) -> Vec<String> {
    catalog_file_search::extract_file_search_paths(files)
}
