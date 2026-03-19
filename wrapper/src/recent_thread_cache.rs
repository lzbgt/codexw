use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::catalog_thread_list::ThreadListEntry;
use crate::catalog_thread_list::ThreadListSnapshot;
use crate::config_persistence::resolve_codex_home;
use crate::output::Output;
use crate::requests::ThreadListView;
use crate::state::AppState;

const RECENT_THREAD_CACHE_FILE: &str = "recent_threads.json";
const RECENT_THREAD_CACHE_VERSION: u32 = 1;
const MAX_CACHED_RECENT_THREADS: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
struct RecentThreadCacheFile {
    version: u32,
    entries: Vec<ThreadListEntry>,
}

fn recent_thread_cache_path(codex_home_override: Option<&Path>) -> Result<std::path::PathBuf> {
    Ok(resolve_codex_home(codex_home_override)?.join(RECENT_THREAD_CACHE_FILE))
}

pub(crate) fn persist_recent_thread_snapshot(
    codex_home_override: Option<&Path>,
    snapshot: &ThreadListSnapshot,
) -> Result<()> {
    let cache_path = recent_thread_cache_path(codex_home_override)?;
    let codex_home = cache_path
        .parent()
        .context("recent thread cache path missing parent directory")?;
    fs::create_dir_all(codex_home).with_context(|| format!("create {}", codex_home.display()))?;
    let contents = serde_json::to_vec_pretty(&RecentThreadCacheFile {
        version: RECENT_THREAD_CACHE_VERSION,
        entries: snapshot
            .entries()
            .iter()
            .take(MAX_CACHED_RECENT_THREADS)
            .cloned()
            .collect(),
    })
    .context("serialize recent thread cache")?;
    fs::write(&cache_path, contents).with_context(|| format!("write {}", cache_path.display()))?;
    Ok(())
}

pub(crate) fn load_recent_thread_snapshot(
    codex_home_override: Option<&Path>,
) -> Result<Option<ThreadListSnapshot>> {
    let cache_path = recent_thread_cache_path(codex_home_override)?;
    let contents = match fs::read(&cache_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {}", cache_path.display())),
    };
    let cache = serde_json::from_slice::<RecentThreadCacheFile>(&contents)
        .with_context(|| format!("parse {}", cache_path.display()))?;
    if cache.version != RECENT_THREAD_CACHE_VERSION {
        return Ok(None);
    }
    Ok(Some(ThreadListSnapshot::from_entries(
        cache
            .entries
            .into_iter()
            .take(MAX_CACHED_RECENT_THREADS)
            .collect(),
    )))
}

pub(crate) fn show_cached_recent_threads(
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let Ok(Some(snapshot)) = load_recent_thread_snapshot(state.codex_home_override.as_deref())
    else {
        return Ok(false);
    };
    if snapshot.is_empty() {
        return Ok(false);
    }
    state.last_listed_thread_ids = snapshot.thread_ids();
    let rendered = snapshot.render(None, ThreadListView::Threads);
    let body = format!(
        "{rendered}\n\n[cached] showing the last local recent-thread snapshot while live thread/list loads."
    );
    output.block_stdout("Threads (cached)", &body)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::load_recent_thread_snapshot;
    use super::persist_recent_thread_snapshot;
    use crate::catalog_thread_list::ThreadListEntry;
    use crate::catalog_thread_list::ThreadListSnapshot;

    #[test]
    fn recent_thread_cache_round_trips_sorted_bounded_entries() {
        let temp = tempfile::tempdir().expect("tempdir");
        let codex_home = temp.path().join("codex-home");
        let snapshot = ThreadListSnapshot::from_entries(vec![
            ThreadListEntry {
                id: "thr-old".to_string(),
                preview: "older".to_string(),
                status: "idle".to_string(),
                updated_at: Some(1),
                cwd: None,
            },
            ThreadListEntry {
                id: "thr-new".to_string(),
                preview: "newer".to_string(),
                status: "active".to_string(),
                updated_at: Some(2),
                cwd: None,
            },
        ]);

        persist_recent_thread_snapshot(Some(&codex_home), &snapshot).expect("persist cache");
        let loaded = load_recent_thread_snapshot(Some(&codex_home))
            .expect("load cache")
            .expect("cache snapshot");

        assert_eq!(loaded.thread_ids(), vec!["thr-new", "thr-old"]);
        assert_eq!(loaded.entries().len(), 2);
        assert_eq!(loaded.entries()[0].status, "active");
    }
}
