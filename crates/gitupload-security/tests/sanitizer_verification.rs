use gitupload_security::sanitizer::sanitize_log_message;

#[test]
fn test_sanitizer_masks_embedded_remote_url() {
    let raw = "git push https://ghp_abcdefghijklmnopqrstuvwxyz123456@github.com/org/repo.git main";
    let cleaned = sanitize_log_message(raw);
    assert!(
        !cleaned.contains("ghp_"),
        "Log line must not contain raw PAT"
    );
    assert!(
        cleaned.contains("https://***@github.com"),
        "URL credentials must be masked with ***"
    );
}

#[test]
fn test_sanitizer_masks_bearer_token() {
    let raw =
        "Authorization: Bearer github_pat_11AAAAAA_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let cleaned = sanitize_log_message(raw);
    assert!(
        !cleaned.contains("github_pat_"),
        "Bearer fine-grained PAT token must be redacted"
    );
}
