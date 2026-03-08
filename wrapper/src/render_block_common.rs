use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BlockKind {
    Markdown,
    Diff,
    Command,
    Thinking,
    Plain,
}

pub(crate) fn classify_block(title: &str, body: &str) -> BlockKind {
    let title = title.to_ascii_lowercase();
    if title.contains("assistant") {
        BlockKind::Markdown
    } else if title.contains("thinking") {
        BlockKind::Thinking
    } else if title.contains("diff")
        || title.contains("file change")
        || body.lines().any(|line| {
            line.starts_with("diff --git ")
                || line.starts_with("@@")
                || line.starts_with("+++ ")
                || line.starts_with("--- ")
        })
    {
        BlockKind::Diff
    } else if title.contains("command") || title.contains("$ command") {
        BlockKind::Command
    } else {
        BlockKind::Plain
    }
}

pub(crate) fn render_title_line(title: &str) -> Line<'static> {
    let accent = if title.eq_ignore_ascii_case("Assistant") {
        Color::Blue
    } else if title.eq_ignore_ascii_case("Thinking") {
        Color::DarkGray
    } else if title.to_ascii_lowercase().contains("diff") {
        Color::Blue
    } else if title.to_ascii_lowercase().contains("command") {
        Color::Green
    } else {
        Color::DarkGray
    };
    Line::from(vec![
        Span::styled("| ", Style::default().fg(accent)),
        Span::styled(
            title.to_string(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
    ])
}

pub(crate) fn style_status_line(line: &str) -> Line<'static> {
    if let Some((tag, rest)) = parse_bracket_tag(line) {
        Line::from(vec![
            Span::styled(
                format!("[{tag}] "),
                Style::default()
                    .fg(match tag {
                        "session" => Color::DarkGray,
                        "interrupt" => Color::Yellow,
                        "turn-error" | "server-error" => Color::Red,
                        "approval" => Color::Magenta,
                        "ready" => Color::Green,
                        _ => Color::Blue,
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else {
        Line::from(Span::raw(line.to_string()))
    }
}

pub(crate) fn split_kv_label(line: &str) -> Option<(&str, &str)> {
    let (label, value) = line.split_once("  ")?;
    Some((label.trim_end(), value))
}

fn parse_bracket_tag(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('[')?;
    let (tag, rest) = rest.split_once(']')?;
    Some((tag, rest.trim_start()))
}
