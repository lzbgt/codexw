use url::Url;

pub(super) fn compose_local_path(base: &Url, local_path: &str) -> String {
    let mut prefix = base.path().trim_end_matches('/').to_string();
    if prefix == "/" {
        prefix.clear();
    }
    format!("{prefix}{local_path}")
}
