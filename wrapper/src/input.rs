#[path = "input/input_build_items.rs"]
mod input_build_items;
#[path = "input/input_build_mentions.rs"]
mod input_build_mentions;
mod input_decode_inline_mentions;
mod input_decode_inline_paths;
mod input_decode_inline_skills;
mod input_decode_mentions;
mod input_decode_tokens;
mod input_resolve_catalog;
mod input_resolve_tools;
mod input_types;

pub use input_types::AppCatalogEntry;
pub use input_types::ParsedInput;
pub use input_types::PluginCatalogEntry;
pub use input_types::SkillCatalogEntry;

use input_build_items::push_attachment_items;
use input_build_items::push_decoded_text_items;
use input_build_mentions::push_catalog_mention_items;

#[cfg(test)]
pub(crate) use input_decode_inline_paths::resolve_file_mention_path;
#[cfg(test)]
pub(crate) use input_decode_mentions::decode_linked_mentions;

pub fn build_turn_input(
    text: &str,
    resolved_cwd: &str,
    pending_local_images: &[String],
    pending_remote_images: &[String],
    apps: &[AppCatalogEntry],
    plugins: &[PluginCatalogEntry],
    skills: &[SkillCatalogEntry],
) -> ParsedInput {
    let preprocessed =
        input_decode_inline_mentions::expand_inline_file_mentions(text, resolved_cwd, plugins);
    let decoded = input_decode_mentions::decode_linked_mentions(&preprocessed);
    let mut items = Vec::new();

    push_attachment_items(&mut items, pending_local_images, pending_remote_images);
    push_decoded_text_items(&mut items, &decoded);
    push_catalog_mention_items(&mut items, &decoded.text, apps, plugins, skills);

    ParsedInput {
        display_text: decoded.text,
        items,
    }
}
