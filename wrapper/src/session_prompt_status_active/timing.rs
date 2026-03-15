use std::time::Instant;

pub(crate) fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let elapsed_millis = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_millis())
        .unwrap_or(0);
    let frame_index = ((elapsed_millis / 80) as usize) % FRAMES.len();
    FRAMES[frame_index]
}

pub(crate) fn format_elapsed(started_at: Option<Instant>) -> String {
    let elapsed = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_secs())
        .unwrap_or(0);
    if elapsed < 60 {
        format!("{elapsed}s")
    } else {
        format!("{}m{:02}s", elapsed / 60, elapsed % 60)
    }
}
