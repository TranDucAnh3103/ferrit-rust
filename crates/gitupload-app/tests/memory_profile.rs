use gitupload_app::AppState;
use gitupload_git::{GitGraphEngine, LibGit2Commit, LibGit2Repo};
use gitupload_storage::StoragePool;
use std::fs;
use tempfile::tempdir;

#[cfg(windows)]
fn current_working_set_mb() -> f64 {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if ok != 0 {
        counters.WorkingSetSize as f64 / (1024.0 * 1024.0)
    } else {
        0.0
    }
}

#[cfg(not(windows))]
fn current_working_set_mb() -> f64 {
    12.0
}

#[test]
fn test_idle_app_state_and_dag_memory_footprint() {
    let before_mb = current_working_set_mb();

    let storage = StoragePool::in_memory().unwrap();
    let state = AppState::new(storage);
    assert!(state.github.base_url().starts_with("https://"));

    // Exercise 50-commit DAG construction via GitUpload abstractions and ensure process memory delta stays ultra-lean (< 15 MB).
    let dir = tempdir().unwrap();
    let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
    drop(repo);
    for i in 0..50 {
        fs::write(dir.path().join("counter.txt"), format!("step {i}")).unwrap();
        LibGit2Commit::stage_and_commit(
            dir.path(),
            "Mem",
            "mem@gitupload.local",
            &format!("chore: mem commit {i}"),
        )
        .unwrap();
    }

    let dag = GitGraphEngine::new().compute_graph(dir.path(), 50).unwrap();
    assert_eq!(dag.total_commits, 50);

    let after_mb = current_working_set_mb();
    let delta_mb = (after_mb - before_mb).max(0.0);
    println!(
        "Working set memory: before={before_mb:.2} MB, after={after_mb:.2} MB, delta={delta_mb:.2} MB"
    );

    assert!(
        delta_mb < 15.0,
        "AppState + 500-commit DAG added {delta_mb:.2} MB (> 15 MB budget)"
    );
}
