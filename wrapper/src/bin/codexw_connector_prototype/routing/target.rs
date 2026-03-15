#[derive(Debug, Clone)]
pub(crate) struct ProxyTarget {
    pub(crate) local_path: String,
    pub(crate) is_sse: bool,
    pub(crate) session_id_hint: Option<String>,
}
