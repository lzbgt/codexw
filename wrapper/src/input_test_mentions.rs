use crate::input::decode_linked_mentions;

#[test]
fn decode_linked_mentions_restores_visible_tokens() {
    let decoded = decode_linked_mentions(
        "Use [$figma](app://figma-1), [$sample](plugin://sample@test), and [$skill](/tmp/demo/SKILL.md).",
    );
    assert_eq!(decoded.text, "Use $figma, $sample, and $skill.");
    assert_eq!(decoded.mentions.len(), 3);
}
