use criterion::{criterion_group, criterion_main, Criterion};
use gitupload_git::{GitGraphEngine, LibGit2Repo};
use tempfile::tempdir;

fn bench_dag_1000_commits(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
    let sig = git2::Signature::now("Bench", "bench@gitupload.local").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();

    let mut prev: Option<git2::Oid> = None;
    for i in 0..1000 {
        let parent = prev.and_then(|o| repo.find_commit(o).ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        let oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("chore: bench commit {i}"),
                &tree,
                &parents,
            )
            .unwrap();
        prev = Some(oid);
    }
    drop(tree);
    drop(repo);

    // Pack the 1,000 loose commits into a standard `.pack` + `.idx` matching real repositories.
    let _ = std::process::Command::new("git")
        .args(["repack", "-ad"])
        .current_dir(dir.path())
        .output();

    let engine = GitGraphEngine::new();
    c.bench_function("dag_revwalk_1000", |b| {
        b.iter(|| {
            let dag = engine.compute_graph(dir.path(), 1000).unwrap();
            assert_eq!(dag.total_commits, 1000);
        });
    });
}

criterion_group!(benches, bench_dag_1000_commits);
criterion_main!(benches);
