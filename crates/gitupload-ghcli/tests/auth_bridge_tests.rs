use gitupload_ghcli::GhAuthBridge;
use gitupload_security::KeyringManager;
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_gh_auth_bridge_bidirectional_ac_p3_02() {
    let dir = tempdir().unwrap();
    let captured_path = dir.path().join("captured_stdin.txt");
    let fake_token = "ghp_MockedGhCliBridgeToken9876543210abcdef";

    #[cfg(windows)]
    let mock_gh_path = {
        let p = dir.path().join("gh.cmd");
        let script = format!(
            "@echo off\r\n\
if \"%1\"==\"auth\" if \"%2\"==\"token\" (\r\n\
  echo {fake_token}\r\n\
  exit /b 0\r\n\
)\r\n\
if \"%1\"==\"auth\" if \"%2\"==\"login\" (\r\n\
  set /p IN_TOK=\r\n\
  > \"{}\" echo !IN_TOK!\r\n\
  exit /b 0\r\n\
)\r\n\
exit /b 1\r\n",
            captured_path.display()
        );
        // Wrap with cmd /V:ON behavior isn't needed if we read stdin directly via `more`:
        let simple_script = format!(
            "@echo off\r\n\
if \"%~1 %~2\"==\"auth token\" (\r\n\
  echo {fake_token}\r\n\
  exit /b 0\r\n\
)\r\n\
if \"%~1 %~2\"==\"auth login\" (\r\n\
  more > \"{}\"\r\n\
  exit /b 0\r\n\
)\r\n\
exit /b 1\r\n",
            captured_path.display()
        );
        let _ = script;
        fs::write(&p, simple_script).unwrap();
        p
    };

    #[cfg(not(windows))]
    let mock_gh_path = {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.path().join("gh");
        let script = format!(
            "#!/bin/sh\n\
if [ \"$1\" = \"auth\" ] && [ \"$2\" = \"token\" ]; then\n\
  echo \"{fake_token}\"\n\
  exit 0\n\
fi\n\
if [ \"$1\" = \"auth\" ] && [ \"$2\" = \"login\" ]; then\n\
  cat > \"{}\"\n\
  exit 0\n\
fi\n\
exit 1\n",
            captured_path.display()
        );
        fs::write(&p, script).unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&p, perms).unwrap();
        p
    };

    // 1. Import token from `gh auth token` into isolated KeyringManager.
    let vault = KeyringManager::new_in_memory();
    let imported = GhAuthBridge::import_token_from_gh(&mock_gh_path, &vault, "stephen")
        .await
        .unwrap();

    assert_eq!(imported.expose_secret(), fake_token);
    // SecretString Debug output must redact secret.
    assert!(!format!("{imported:?}").contains(fake_token));

    let vault_key = KeyringManager::github_account_key("stephen");
    let loaded = vault.get_token(&vault_key).unwrap().unwrap();
    assert_eq!(loaded.expose_secret(), fake_token);

    // 2. Export PAT from Keyring into `gh auth login --with-token` via stdin pipe.
    let pat_out = SecretString::from("ghp_ExportedSecretTokenForGhCli123456".to_string());
    GhAuthBridge::export_token_to_gh(&mock_gh_path, &pat_out)
        .await
        .unwrap();

    let captured = fs::read_to_string(&captured_path).unwrap();
    assert!(captured
        .trim()
        .contains("ghp_ExportedSecretTokenForGhCli123456"));
}
