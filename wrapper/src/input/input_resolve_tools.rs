use std::collections::HashMap;
use std::collections::HashSet;

use crate::input::input_decode::is_common_env_var;
use crate::input::input_decode::is_mention_name_char;
use crate::input::input_decode::mention_skill_path;
use crate::input::input_decode::parse_linked_tool_mention;

#[derive(Debug, Default, Clone)]
pub(crate) struct ToolMentions {
    pub names: HashSet<String>,
    pub linked_paths: HashMap<String, String>,
}

pub(crate) fn collect_tool_mentions(text: &str) -> ToolMentions {
    let bytes = text.as_bytes();
    let mut names = HashSet::new();
    let mut linked_paths = HashMap::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((name, path, end_index)) = parse_linked_tool_mention(text, bytes, index)
        {
            if !is_common_env_var(name) {
                if mention_skill_path(path).is_some() {
                    names.insert(name.to_ascii_lowercase());
                }
                linked_paths
                    .entry(name.to_ascii_lowercase())
                    .or_insert_with(|| path.to_string());
            }
            index = end_index;
            continue;
        }

        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let name_start = index + 1;
        let Some(first) = bytes.get(name_start) else {
            index += 1;
            continue;
        };
        if !is_mention_name_char(*first) {
            index += 1;
            continue;
        }
        let mut name_end = name_start + 1;
        while let Some(next) = bytes.get(name_end)
            && is_mention_name_char(*next)
        {
            name_end += 1;
        }
        let name = &text[name_start..name_end];
        if !is_common_env_var(name) {
            names.insert(name.to_ascii_lowercase());
        }
        index = name_end;
    }

    ToolMentions {
        names,
        linked_paths,
    }
}
