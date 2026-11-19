use gitupload_security::PlatformVault;
use secrecy::{ExposeSecret, SecretString};

#[tokio::test]
async fn test_platform_vault_roundtrip() {
    let vault = PlatformVault::ephemeral_in_memory();
    assert!(!PlatformVault::active_backend_name().is_empty());

    let account = "test_user_matrix";
    let token = SecretString::from("ghp_SecretTokenRoundtrip1234567890".to_string());

    vault.store(account, &token).await.unwrap();
    let retrieved = vault.retrieve(account).await.unwrap();
    assert_eq!(
        retrieved.expose_secret(),
        "ghp_SecretTokenRoundtrip1234567890"
    );

    vault.delete(account).await.unwrap();
    assert!(vault.retrieve(account).await.is_err());
}
