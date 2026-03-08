use std::sync::LazyLock;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static SYNTAX_THEME: LazyLock<Theme> = LazyLock::new(|| {
    let themes = ThemeSet::load_defaults();
    themes
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .unwrap_or_default()
});

pub(crate) fn render_code_block(language: &str, code: &str) -> Text<'static> {
    let syntax = syntax_for_language(language);
    let mut highlighter = HighlightLines::new(syntax, &SYNTAX_THEME);
    let mut lines = Vec::new();
    if !language.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            format!("```{}", language.trim()),
            Style::default().fg(Color::DarkGray),
        )));
    }
    for line in LinesWithEndings::from(code) {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => {
                let spans = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(
                            text.replace('\n', ""),
                            Style::default()
                                .fg(Color::Rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                ))
                                .bg(Color::Rgb(
                                    style.background.r,
                                    style.background.g,
                                    style.background.b,
                                )),
                        )
                    })
                    .collect::<Vec<_>>();
                lines.push(Line::from(spans));
            }
            Err(_) => lines.push(Line::from(Span::raw(
                line.trim_end_matches('\n').to_string(),
            ))),
        }
    }
    if !language.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            "```",
            Style::default().fg(Color::DarkGray),
        )));
    }
    Text::from(lines)
}

pub(crate) fn highlight_code_line(language: &str, line: &str) -> Line<'static> {
    let syntax = syntax_for_language(language);
    let mut highlighter = HighlightLines::new(syntax, &SYNTAX_THEME);
    match highlighter.highlight_line(line, &SYNTAX_SET) {
        Ok(ranges) => Line::from(
            ranges
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(
                        text.to_string(),
                        Style::default().fg(Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        )),
                    )
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => Line::from(Span::raw(line.to_string())),
    }
}

fn syntax_for_language(language: &str) -> &'static SyntaxReference {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return SYNTAX_SET.find_syntax_plain_text();
    }
    SYNTAX_SET
        .find_syntax_by_token(trimmed)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(trimmed))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}
