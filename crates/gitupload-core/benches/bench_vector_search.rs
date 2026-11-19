use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gitupload_core::VectorSearchEngine;

fn generate_mock_vectors(count: usize, dims: usize) -> Vec<(String, String, Vec<f32>)> {
    let mut list = Vec::with_capacity(count);
    for i in 0..count {
        let oid = format!("{i:040x}");
        let summary = format!("Commit message summary for commit index {i}");
        let vec: Vec<f32> = (0..dims).map(|d| ((i + d) % 100) as f32 / 100.0).collect();
        list.push((oid, summary, vec));
    }
    list
}

fn bench_cosine_search_10k(c: &mut Criterion) {
    let dataset = generate_mock_vectors(10_000, 512);
    let query_vector: Vec<f32> = (0..512).map(|d| (d % 50) as f32 / 50.0).collect();

    c.bench_function("vector_search_10k_commits_512d", |b| {
        b.iter(|| {
            let hits = VectorSearchEngine::rank_matches(
                black_box(&query_vector),
                black_box(&dataset),
                black_box(20),
                black_box(0.50),
            );
            assert_eq!(hits.len(), 20);
        });
    });
}

criterion_group!(benches, bench_cosine_search_10k);
criterion_main!(benches);
