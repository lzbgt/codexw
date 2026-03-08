use super::input_types::DecodedHistoryText;
use super::input_types::LinkedMention;

pub fn decode_linked_mentions(text: &str) -> DecodedHistoryText {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut mentions = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((name, path, end_index)) = parse_linked_tool_mention(text, bytes, index)
            && !super::input_decode_tokens::is_common_env_var(name)
            && is_tool_path(path)
        {
            out.push('$');
            out.push_str(name);
            mentions.push(LinkedMention {
                mention: name.to_string(),
                path: path.to_string(),
            });
            index = end_index;
            continue;
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }

    DecodedHistoryText {
        text: out,
        mentions,
    }
}
#[path = "input_decode_mention_links.rs"]
mod input_decode_mention_links;
#[path = "input_decode_mention_paths.rs"]
mod input_decode_mention_paths;

pub(crate) use input_decode_mention_links::parse_linked_tool_mention;
pub(crate) use input_decode_mention_paths::is_tool_path;
