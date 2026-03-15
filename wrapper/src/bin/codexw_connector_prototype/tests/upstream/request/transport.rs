use crate::upstream::compose_local_path;

#[test]
fn compose_local_path_preserves_local_api_base_prefix() {
    let base = url::Url::parse("http://127.0.0.1:8080/base/v1/").expect("url");
    assert_eq!(
        compose_local_path(&base, "/api/v1/session/sess_1"),
        "/base/v1/api/v1/session/sess_1"
    );
}
