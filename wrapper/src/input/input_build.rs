#[path = "input_build_items.rs"]
mod input_build_items;
#[path = "input_build_mentions.rs"]
mod input_build_mentions;

use super::input_decode_inline::expand_inline_file_mentions;
use super::input_decode_mentions::decode_linked_mentions;
use super::input_types::AppCatalogEntry;
use super::input_types::ParsedInput;
use super::input_types::PluginCatalogEntry;
use super::input_types::SkillCatalogEntry;
use input_build_items::push_attachment_items;
use input_build_items::push_decoded_text_items;
use input_build_mentions::push_catalog_mention_items;

pub fn build_turn_input(
    text: &str,
    resolved_cwd: &str,
    pending_local_images: &[String],
    pending_remote_images: &[String],
    apps: &[AppCatalogEntry],
    plugins: &[PluginCatalogEntry],
    skills: &[SkillCatalogEntry],
) -> ParsedInput {
    let preprocessed = expand_inline_file_mentions(text, resolved_cwd, plugins);
    let decoded = decode_linked_mentions(&preprocessed);
    let mut items = Vec::new();

    push_attachment_items(&mut items, pending_local_images, pending_remote_images);
    push_decoded_text_items(&mut items, &decoded);
    push_catalog_mention_items(&mut items, &decoded.text, apps, plugins, skills);

    ParsedInput {
        display_text: decoded.text,
        items,
    }
}
