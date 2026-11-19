use git2::{Repository, Signature};
use gitupload_core::models::{
    RemoteEndpoint, RemoteHealthStatus, RemoteProviderType, SyncPolicy, SyncStatus,
};
use gitupload_git::{MultiRemoteCoordinator, RemoteHealthChecker, SafePushMode};
use gitupload_security::{KeyringManager, RemoteVaultResolver};
use std::sync::Arc;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_multi_remote_parallel_mirror_and_isolation() {
    let dir = tempdir().unwrap();
    let work_dir = dir.path().join("local_repo");
    let bare_alpha = dir.path().join("bare_alpha.git");
    let bare_beta = dir.path().join("bare_beta.git");

    Repository::init_bare(&bare_alpha).unwrap();
    Repository::init_bare(&bare_beta).unwrap();

    let repo = Repository::init(&work_dir).unwrap();
    std::fs::write(work_dir.join("README.md"), "# Multi-Remote Test\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Stephen", "stephen@gitupload.local").unwrap();
    repo.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "feat: initial commit",
        &tree,
        &[],
    )
    .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    drop(tree);
    drop(repo);

    let endpoints = vec![
        RemoteEndpoint {
            name: "alpha".to_string(),
            url: bare_alpha.to_string_lossy().to_string(),
            provider: RemoteProviderType::GitHub,
            sync_policy: SyncPolicy::Bidirectional,
            auth_account: "acct-a".to_string(),
            is_enabled: true,
            timeout_seconds: 10,
        },
        RemoteEndpoint {
            name: "beta".to_string(),
            url: bare_beta.to_string_lossy().to_string(),
            provider: RemoteProviderType::GitLab,
            sync_policy: SyncPolicy::PushOnly,
            auth_account: "acct-b".to_string(),
            is_enabled: true,
            timeout_seconds: 10,
        },
        RemoteEndpoint {
            name: "broken-remote".to_string(),
            url: "file:///nonexistent_invalid_remote_path_xyz.git".to_string(),
            provider: RemoteProviderType::Custom,
            sync_policy: SyncPolicy::BackupMirror,
            auth_account: "".to_string(),
            is_enabled: true,
            timeout_seconds: 2,
        },
        RemoteEndpoint {
            name: "disabled-remote".to_string(),
            url: "https://example.com/disabled.git".to_string(),
            provider: RemoteProviderType::Gitea,
            sync_policy: SyncPolicy::BackupMirror,
            auth_account: "".to_string(),
            is_enabled: false,
            timeout_seconds: 5,
        },
    ];

    let vault = RemoteVaultResolver::with_keyring(Arc::new(KeyringManager::new_in_memory()));
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    let results = MultiRemoteCoordinator::push_all(
        &work_dir,
        "main",
        &endpoints,
        SafePushMode::StrictSafe,
        &vault,
        Some(tx),
        CancellationToken::new(),
    )
    .await;

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].remote_name, "alpha");
    assert!(
        results[0].is_success,
        "alpha failed: {:?}",
        results[0].error_message
    );
    assert_eq!(results[0].status, SyncStatus::Success);

    assert_eq!(results[1].remote_name, "beta");
    assert!(
        results[1].is_success,
        "beta failed: {:?}",
        results[1].error_message
    );
    assert_eq!(results[1].status, SyncStatus::Success);

    assert_eq!(results[2].remote_name, "broken-remote");
    assert!(!results[2].is_success);
    assert_eq!(results[2].status, SyncStatus::Failed);

    assert_eq!(results[3].remote_name, "disabled-remote");
    assert!(results[3].is_success);
    assert_eq!(results[3].status, SyncStatus::Skipped);

    // Verify bare mirrors actually received refs/heads/main
    let alpha_repo = Repository::open_bare(&bare_alpha).unwrap();
    assert!(alpha_repo.find_reference("refs/heads/main").is_ok());
    let beta_repo = Repository::open_bare(&bare_beta).unwrap();
    assert!(beta_repo.find_reference("refs/heads/main").is_ok());

    // Ensure progress channel emitted aggregate updates ending at 100%
    let mut last_pct = 0;
    while let Ok(evt) = rx.try_recv() {
        last_pct = evt.overall_percentage;
    }
    assert_eq!(last_pct, 100);

    // Probe healthy vs unreachable endpoint
    let probe_ok = RemoteHealthChecker::probe(&endpoints[0], &vault).await;
    assert_eq!(probe_ok.status, RemoteHealthStatus::Healthy);
    let probe_fail = RemoteHealthChecker::probe(&endpoints[2], &vault).await;
    assert_eq!(probe_fail.status, RemoteHealthStatus::Unreachable);
}
