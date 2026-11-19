use criterion::{criterion_group, criterion_main, Criterion};
use gitupload_hooks::{HookRunner, HooksConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn bench_hook_validation(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let mut files: Vec<PathBuf> = Vec::with_capacity(50);
    for i in 0..50 {
        let rel = PathBuf::from(format!("src_{i:02}.rs"));
        fs::write(dir.path().join(&rel), "pub fn sample() -> u32 { 42 }\n").unwrap();
        files.push(rel);
    }

    let cfg = HooksConfig::default();

    c.bench_function("hook_conventional_commit_msg", |b| {
        b.iter(|| {
            HookRunner::validate_commit_msg("feat(git): optimize DAG revwalk performance", 72)
                .unwrap();
        });
    });

    c.bench_function("hook_pre_commit_50_files", |b| {
        b.iter(|| {
            let report = HookRunner::scan_pre_commit(dir.path(), &files, &cfg.pre_commit).unwrap();
            assert!(report.blocked_env_files.is_empty() && report.secret_findings.is_empty());
        });
    });
}

criterion_group!(benches, bench_hook_validation);
criterion_main!(benches);
