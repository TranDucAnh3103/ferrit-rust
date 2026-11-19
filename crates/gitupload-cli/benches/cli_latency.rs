use criterion::{criterion_group, criterion_main, Criterion};
use std::process::Command;

fn bench_cli_version_startup(c: &mut Criterion) {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");

    c.bench_function("cli_version_cold_start", |b| {
        b.iter(|| {
            let output = Command::new(cli_path)
                .arg("--version")
                .output()
                .expect("Failed to execute CLI");
            assert!(output.status.success());
        });
    });
}

fn bench_cli_help_cold_start(c: &mut Criterion) {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");

    c.bench_function("cli_help_cold_start", |b| {
        b.iter(|| {
            let output = Command::new(cli_path)
                .arg("--help")
                .output()
                .expect("Failed to execute CLI");
            assert!(output.status.success());
        });
    });
}

fn bench_cli_completions_latency(c: &mut Criterion) {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");

    c.bench_function("cli_completions_bash", |b| {
        b.iter(|| {
            let output = Command::new(cli_path)
                .args(["completions", "bash"])
                .output()
                .expect("Failed to execute completions");
            assert!(output.status.success());
        });
    });
}

criterion_group!(
    benches,
    bench_cli_version_startup,
    bench_cli_help_cold_start,
    bench_cli_completions_latency
);
criterion_main!(benches);
