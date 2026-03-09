use crate::catalog_connector_views::render_apps_list;
use crate::catalog_feature_views::render_experimental_features_list;
use crate::input::AppCatalogEntry;
use serde_json::json;

#[test]
fn app_list_rendering_includes_slug_and_status() {
    let rendered = render_apps_list(&[
        AppCatalogEntry {
            id: "connector_1".to_string(),
            name: "Demo App".to_string(),
            slug: "demo-app".to_string(),
            enabled: true,
        },
        AppCatalogEntry {
            id: "connector_2".to_string(),
            name: "Hidden App".to_string(),
            slug: "hidden-app".to_string(),
            enabled: false,
        },
    ]);
    assert!(rendered.contains("$demo-app"));
    assert!(rendered.contains("[enabled]"));
    assert!(rendered.contains("[disabled]"));
}

#[test]
fn experimental_feature_rendering_shows_stage_status_and_key() {
    let rendered = render_experimental_features_list(&json!({
        "data": [
            {
                "name": "background-terminals",
                "displayName": "Background terminals",
                "enabled": true,
                "stage": "beta",
                "description": "Run background terminal sessions",
                "announcement": "This feature is in beta"
            }
        ]
    }));
    assert!(rendered.contains("Background terminals"));
    assert!(rendered.contains("[enabled]"));
    assert!(rendered.contains("[beta]"));
    assert!(rendered.contains("key: background-terminals"));
    assert!(rendered.contains("Run background terminal sessions"));
}
