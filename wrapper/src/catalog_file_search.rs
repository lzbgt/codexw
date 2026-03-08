use serde_json::Value;

use crate::state::get_string;

pub(crate) fn render_fuzzy_file_search_results(query: &str, files: &[Value]) -> String {
    if files.is_empty() {
        return format!("No files matched \"{query}\".");
    }
    let mut lines = vec![format!("Query: {query}")];
    for (index, file) in files.iter().take(20).enumerate() {
        let path = get_string(file, &["path"]).unwrap_or("?");
        let score = file
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        lines.push(format!("{:>2}. {}  [score {}]", index + 1, path, score));
    }
    if files.len() > 20 {
        lines.push(format!("...and {} more", files.len() - 20));
    }
    lines.push("Use /mention <n> to insert a match into the prompt.".to_string());
    lines.join("\n")
}

pub(crate) fn extract_file_search_paths(files: &[Value]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| get_string(file, &["path"]).map(ToOwned::to_owned))
        .collect()
}
