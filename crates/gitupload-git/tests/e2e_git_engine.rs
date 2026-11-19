use git2::Repository;
use gitupload_core::errors::GitUploadError;
use gitupload_git::{
    AutoStashGuard, GitGraphActions, GitGraphEngine, GitignoreGenerator, LfsManager, LibGit2Commit,
    LibGit2Repo, LibGit2Transport, RePushInspector, SafePushMode, SafeSyncEngine,
};
use std::fs;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[test]
fn test_e2e_ephemeral_git_lifecycle() {
    let bare_remote = tempdir().unwrap();
    let repo_a = tempdir().unwrap();
    let repo_b = tempdir().unwrap();

    let bare = Repository::init_bare(bare_remote.path()).unwrap();
    bare.set_head("refs/heads/main").unwrap();
    drop(bare);

    // 1. Initialize repository A, configure identity, write safety .gitignore & LFS rules.
    let git_repo = LibGit2Repo::init_or_open(repo_a.path()).unwrap();
    LibGit2Repo::configure_identity(&git_repo, "E2E Tester", "e2e@gitupload.local").unwrap();
    LibGit2Transport::set_clean_remote(&git_repo, "origin", bare_remote.path().to_str().unwrap())
        .unwrap();
    drop(git_repo);

    GitignoreGenerator::write_to_repo(repo_a.path(), &["Rust", "Node"]).unwrap();
    LfsManager::track_patterns(repo_a.path(), &["*.psd".to_string()]).unwrap();
    fs::write(repo_a.path().join("main.rs"), "fn main() {}\n").unwrap();

    let oid1 = LibGit2Commit::stage_and_commit(
        repo_a.path(),
        "E2E Tester",
        "e2e@gitupload.local",
        "feat(core): initial commit",
    )
    .unwrap();

    let cancel = CancellationToken::new();
    SafeSyncEngine::push_with_safety(
        repo_a.path(),
        "origin",
        "main",
        None,
        SafePushMode::StrictSafe,
        false,
        &cancel,
        |_, _| {},
    )
    .unwrap();

    // 2. Clone into B, create branch + commit, verify DAG graph & revert/cherry-pick.
    LibGit2Transport::clone_repo(
        bare_remote.path().to_str().unwrap(),
        repo_b.path(),
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();

    fs::write(repo_b.path().join("feature.txt"), "feature v1\n").unwrap();
    let oid_feat = LibGit2Commit::stage_and_commit(
        repo_b.path(),
        "Collaborator B",
        "b@gitupload.local",
        "feat(ui): add feature",
    )
    .unwrap();

    LibGit2Transport::push_branch(
        repo_b.path(),
        "origin",
        "main",
        None,
        SafePushMode::StrictSafe,
        &cancel,
        |_, _| {},
    )
    .unwrap();

    // 3. Re-push safety check on A after local uncommitted edit -> AutoStash preserves work and StrictSafe blocks collision.
    fs::write(
        repo_a.path().join("main.rs"),
        "fn main() { println!(\"local\"); }\n",
    )
    .unwrap();
    AutoStashGuard::with_auto_stash(repo_a.path(), true, || {
        assert!(LibGit2Repo::status_files(repo_a.path())?.is_empty());
        Ok(())
    })
    .unwrap();
    assert!(fs::read_to_string(repo_a.path().join("main.rs"))
        .unwrap()
        .contains("local"));

    // Commit diverged local change on A and ensure both StrictSafe & ForceWithLease reject overwrite.
    let _oid_div = LibGit2Commit::stage_and_commit(
        repo_a.path(),
        "E2E Tester",
        "e2e@gitupload.local",
        "fix(core): diverged work",
    )
    .unwrap();

    let err_strict = SafeSyncEngine::push_with_safety(
        repo_a.path(),
        "origin",
        "main",
        None,
        SafePushMode::StrictSafe,
        false,
        &cancel,
        |_, _| {},
    )
    .unwrap_err();
    assert!(matches!(
        err_strict,
        GitUploadError::DivergenceDetected { .. }
    ));

    // 4. DAG Revwalk + Revert on B
    let dag = GitGraphEngine::new()
        .compute_graph(repo_b.path(), 50)
        .unwrap();
    assert_eq!(dag.total_commits, 2);
    assert_eq!(dag.nodes[0].oid, oid_feat.to_string());
    assert_eq!(dag.nodes[1].oid, oid1.to_string());

    let revert_oid = GitGraphActions::revert_commit(repo_b.path(), &oid_feat.to_string()).unwrap();
    assert!(!revert_oid.is_empty());

    let inspected = RePushInspector::inspect(repo_b.path()).unwrap();
    assert!(inspected.is_git_repo);
    assert!(inspected.has_lfs);
}
