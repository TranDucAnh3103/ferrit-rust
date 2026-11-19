use git2::{Repository, Signature};
use gitupload_core::models::{BranchMergeStatus, PruneBranchesRequest};
use gitupload_git::{BranchPruner, CommitIndexer, GitBackupEngine, StaleBranchAnalyzer};
use std::collections::HashSet;
use tempfile::tempdir;

fn setup_test_repo_with_merged_branch() -> (tempfile::TempDir, Repository) {
    let dir = tempdir().expect("Failed to create tempdir");
    let repo = Repository::init(dir.path()).expect("Failed to init repo");
    let sig = Signature::now("Tester", "test@gitupload.local").unwrap();

    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let initial_oid = repo
        .commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "feat: initial main",
            &tree,
            &[],
        )
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    let initial_commit = repo.find_commit(initial_oid).unwrap();

    // Create feature branch with one commit, then advance main to simulate merge
    repo.branch("feature/test", &initial_commit, false).unwrap();
    let feat_oid = repo
        .commit(
            Some("refs/heads/feature/test"),
            &sig,
            &sig,
            "feat: implement branch cleaner",
            &tree,
            &[&initial_commit],
        )
        .unwrap();

    let mut main_ref = repo.find_reference("refs/heads/main").unwrap();
    main_ref.set_target(feat_oid, "Merge feature").unwrap();
    drop(main_ref);

    // Also create unmerged branch
    let feat_commit = repo.find_commit(feat_oid).unwrap();
    repo.branch("feature/wip", &feat_commit, false).unwrap();
    repo.commit(
        Some("refs/heads/feature/wip"),
        &sig,
        &sig,
        "wip: unmerged change",
        &tree,
        &[&feat_commit],
    )
    .unwrap();

    drop(feat_commit);
    drop(initial_commit);
    drop(tree);
    (dir, repo)
}

#[test]
fn test_analyzer_detects_merged_branch_and_collects_unindexed_commits() {
    let (dir, _repo) = setup_test_repo_with_merged_branch();
    let results =
        StaleBranchAnalyzer::analyze_local_branches(dir.path().to_str().unwrap(), "main").unwrap();

    let feature = results
        .iter()
        .find(|b| b.name == "feature/test")
        .expect("Merged feature branch must be listed");
    assert_eq!(feature.merge_status, BranchMergeStatus::FullyMerged);

    let wip = results
        .iter()
        .find(|b| b.name == "feature/wip")
        .expect("WIP branch must be listed");
    assert_eq!(wip.merge_status, BranchMergeStatus::UnmergedAhead);

    let empty = HashSet::new();
    let commits = CommitIndexer::collect_unindexed_commits(dir.path(), &empty, 50).unwrap();
    assert_eq!(commits.len(), 2);
}

#[tokio::test]
async fn test_pruner_blocks_protected_branches_and_creates_bundle() {
    let (dir, _repo) = setup_test_repo_with_merged_branch();

    let req = PruneBranchesRequest {
        repo_path: dir.path().to_str().unwrap().to_string(),
        branch_names: vec![
            "main".into(),
            "master".into(),
            "release/v1.0".into(),
            "feature/test".into(),
        ],
        delete_remote: false,
        prune_tracking_refs: false,
    };

    let result = BranchPruner::execute_prune(&req).await.unwrap();
    assert_eq!(result.deleted_local_count, 1);
    assert_eq!(result.failed_deletions.len(), 3);

    let backup_dir = dir.path().join("backups");
    let bundle_path = GitBackupEngine::create_repo_bundle(dir.path(), &backup_dir, "demo")
        .await
        .unwrap();
    assert!(bundle_path.exists());
}
