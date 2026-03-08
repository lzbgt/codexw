use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedMention {
    pub mention: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInput {
    pub display_text: String,
    pub items: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCatalogEntry {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCatalogEntry {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCatalogEntry {
    pub name: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHistoryText {
    pub text: String,
    pub mentions: Vec<LinkedMention>,
}
