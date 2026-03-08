use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Text;

pub(crate) fn text_to_ansi(text: &Text<'_>) -> String {
    text.lines
        .iter()
        .map(line_to_ansi)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn line_to_ansi(line: &Line<'_>) -> String {
    let mut out = String::new();
    for span in &line.spans {
        out.push_str(&style_start(span.style));
        out.push_str(span.content.as_ref());
        out.push_str("\x1b[0m");
    }
    out
}

fn style_start(style: Style) -> String {
    let mut codes = Vec::new();
    if style.add_modifier.contains(Modifier::BOLD) {
        codes.push("1".to_string());
    }
    if style.add_modifier.contains(Modifier::DIM) {
        codes.push("2".to_string());
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        codes.push("3".to_string());
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        codes.push("4".to_string());
    }
    if style.add_modifier.contains(Modifier::REVERSED) {
        codes.push("7".to_string());
    }
    if style.add_modifier.contains(Modifier::CROSSED_OUT) {
        codes.push("9".to_string());
    }
    if let Some(fg) = style.fg {
        codes.push(color_code(fg, false));
    }
    if let Some(bg) = style.bg {
        codes.push(color_code(bg, true));
    }
    if codes.is_empty() {
        "\x1b[0m".to_string()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn color_code(color: Color, background: bool) -> String {
    match color {
        Color::Reset => {
            if background {
                "49".to_string()
            } else {
                "39".to_string()
            }
        }
        Color::Black => basic_color_code(0, background),
        Color::Red => basic_color_code(1, background),
        Color::Green => basic_color_code(2, background),
        Color::Yellow => basic_color_code(3, background),
        Color::Blue => basic_color_code(4, background),
        Color::Magenta => basic_color_code(5, background),
        Color::Cyan => basic_color_code(6, background),
        Color::Gray | Color::White => basic_color_code(7, background),
        Color::DarkGray => bright_color_code(0, background),
        Color::LightRed => bright_color_code(1, background),
        Color::LightGreen => bright_color_code(2, background),
        Color::LightYellow => bright_color_code(3, background),
        Color::LightBlue => bright_color_code(4, background),
        Color::LightMagenta => bright_color_code(5, background),
        Color::LightCyan => bright_color_code(6, background),
        Color::Rgb(r, g, b) => format!("{};2;{};{};{}", if background { 48 } else { 38 }, r, g, b),
        Color::Indexed(index) => format!("{};5;{}", if background { 48 } else { 38 }, index),
    }
}

fn basic_color_code(index: u8, background: bool) -> String {
    format!("{}", if background { 40 + index } else { 30 + index })
}

fn bright_color_code(index: u8, background: bool) -> String {
    format!("{}", if background { 100 + index } else { 90 + index })
}
