use git2::{Repository, Signature};
use gitupload_core::models::{ConflictResolutionChoice, FileResolutionRequest};
use gitupload_git::{CommitBuilder, ConflictParser, ConflictResolver, MergeCalculator};
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_three_way_merge_unicode_and_atomic_commit() {
    let dir = tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let sig = Signature::now("Stephen", "stephen@gitupload.local").unwrap();

    // Base commit with shared prefix/suffix and Vietnamese UTF-8 content
    let base_src = "fn header() {}\nlet greeting = \"Xin chao\";\nfn footer() {}\n";
    std::fs::write(dir.path().join("app.rs"), base_src).unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(std::path::Path::new("app.rs")).unwrap();
    let base_tree_id = idx.write_tree().unwrap();
    let base_tree = repo.find_tree(base_tree_id).unwrap();
    let base_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "chore: base", &base_tree, &[])
        .unwrap();
    let base_commit = repo.find_commit(base_oid).unwrap();

    // Create incoming branch `feature/i18n` with diverging middle line + binary asset
    repo.branch("feature/i18n", &base_commit, false).unwrap();
    repo.set_head("refs/heads/feature/i18n").unwrap();
    let theirs_src =
        "fn header() {}\nlet greeting = \"Xin chao the gioi — 日本語\";\nfn footer() {}\n";
    std::fs::write(dir.path().join("app.rs"), theirs_src).unwrap();
    let mut idx_theirs = repo.index().unwrap();
    idx_theirs.add_path(std::path::Path::new("app.rs")).unwrap();
    let theirs_tree_id = idx_theirs.write_tree().unwrap();
    let theirs_tree = repo.find_tree(theirs_tree_id).unwrap();
    let _theirs_oid = repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "feat: i18n greeting",
            &theirs_tree,
            &[&base_commit],
        )
        .unwrap();

    // Switch back to main/master and commit conflicting local change
    repo.set_head("refs/heads/master")
        .or_else(|_| repo.set_head("refs/heads/main"))
        .unwrap_or_else(|_| {
            repo.branch("main", &base_commit, false).unwrap();
            repo.set_head("refs/heads/main").unwrap();
        });
    let ours_src = "fn header() {}\nlet greeting = \"Hello Rust Desktop\";\nfn footer() {}\n";
    std::fs::write(dir.path().join("app.rs"), ours_src).unwrap();
    let mut idx_ours = repo.index().unwrap();
    idx_ours.add_path(std::path::Path::new("app.rs")).unwrap();
    let ours_tree_id = idx_ours.write_tree().unwrap();
    let ours_tree = repo.find_tree(ours_tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "feat: local greeting",
        &ours_tree,
        &[&base_commit],
    )
    .unwrap();
    drop(ours_tree);
    drop(theirs_tree);
    drop(base_tree);
    drop(base_commit);
    drop(repo);

    let session = MergeCalculator::detect_conflicts(dir.path(), "feature/i18n").unwrap();
    assert_eq!(session.conflicting_files.len(), 1);
    let file = &session.conflicting_files[0];
    assert_eq!(file.file_path, "app.rs");
    assert!(!file.is_binary);
    assert_eq!(file.hunks.len(), 1);
    assert!(file.hunks[0].content_theirs.contains("Xin chao the gioi"));
    assert!(file.hunks[0].content_base.is_some());

    let req = FileResolutionRequest {
        file_path: "app.rs".to_string(),
        chunk_resolutions: vec![(0, ConflictResolutionChoice::AcceptBoth)],
        custom_content: None,
    };
    let preview =
        ConflictResolver::apply_resolutions(&file.raw_conflicted_text, &file.hunks, &req).unwrap();
    assert!(preview.contains("Hello Rust Desktop"));
    assert!(preview.contains("Xin chao the gioi — 日本語"));
    assert!(!preview.contains("<<<<<<<"));

    let merge_oid = CommitBuilder::finalize_merge(
        dir.path(),
        "feature/i18n",
        &[req],
        Some("chore(merge): reconcile i18n greeting"),
    )
    .unwrap();

    let verify_repo = Repository::open(dir.path()).unwrap();
    let merge_commit = verify_repo
        .find_commit(git2::Oid::from_str(&merge_oid).unwrap())
        .unwrap();
    assert_eq!(merge_commit.parent_count(), 2);
}

#[test]
fn test_1000_randomized_conflict_parse_and_resolve_stress() {
    let mut template = String::new();
    for i in 0..1000 {
        template.push_str(&format!(
            "line_before_{i}\n<<<<<<< HEAD\nlet v_{i} = \"ours_{i}_tieng_viet\";\n||||||| base\nlet v_{i} = \"base_{i}\";\n=======\nlet v_{i} = \"theirs_{i}_日本語\";\n>>>>>>> feature\nline_after_{i}\n"
        ));
    }

    let started = Instant::now();
    let hunks = ConflictParser::parse("stress_big.rs", &template);
    assert_eq!(hunks.len(), 1000);

    let choices: Vec<(usize, ConflictResolutionChoice)> = (0..1000)
        .map(|i| {
            let c = match i % 3 {
                0 => ConflictResolutionChoice::AcceptCurrent,
                1 => ConflictResolutionChoice::AcceptIncoming,
                _ => ConflictResolutionChoice::AcceptBoth,
            };
            (i, c)
        })
        .collect();

    let resolved = ConflictResolver::apply_resolutions(
        &template,
        &hunks,
        &FileResolutionRequest {
            file_path: "stress_big.rs".to_string(),
            chunk_resolutions: choices,
            custom_content: None,
        },
    )
    .unwrap();
    let elapsed = started.elapsed();

    assert!(!resolved.contains("<<<<<<<"));
    assert!(!resolved.contains("======="));
    assert!(!resolved.contains(">>>>>>>"));
    assert!(
        elapsed.as_millis() < 150,
        "1000-hunk parse+resolve took {:?}",
        elapsed
    );
}
