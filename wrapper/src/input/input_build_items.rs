use serde_json::Value;
use serde_json::json;

use crate::input::input_decode_inline_skills::mention_skill_path;
use crate::input::input_types::DecodedHistoryText;

pub(crate) fn push_attachment_items(
    items: &mut Vec<Value>,
    pending_local_images: &[String],
    pending_remote_images: &[String],
) {
    for url in pending_remote_images {
        items.push(json!({
            "type": "image",
            "url": url,
        }));
    }

    for path in pending_local_images {
        items.push(json!({
            "type": "localImage",
            "path": path,
        }));
    }
}

pub(crate) fn push_decoded_text_items(items: &mut Vec<Value>, decoded: &DecodedHistoryText) {
    if !decoded.text.trim().is_empty() {
        items.push(json!({
            "type": "text",
            "text": decoded.text,
            "text_elements": [],
        }));
    }

    for mention in &decoded.mentions {
        if let Some(skill_path) = mention_skill_path(&mention.path) {
            items.push(json!({
                "type": "skill",
                "name": mention.mention,
                "path": skill_path,
            }));
        } else {
            items.push(json!({
                "type": "mention",
                "name": mention.mention,
                "path": mention.path,
            }));
        }
    }
}
