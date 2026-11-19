use gitupload_core::models::{SecurityAuditReport, VulnerabilityFinding, VulnerabilitySeverity};
use gitupload_core::VectorSearchEngine;
use gitupload_storage::{EmbeddingStore, StoragePool};

#[test]
fn test_embedding_blob_roundtrip_and_vector_ranking() {
    let pool = StoragePool::in_memory().unwrap();
    let store = EmbeddingStore::new(&pool);

    let vec_auth = vec![0.9_f32, 0.1_f32, 0.0_f32];
    let vec_ui = vec![0.0_f32, 0.9_f32, 0.1_f32];

    store
        .save_embedding(
            "C:\\repo\\app",
            "1111aaaa",
            "fix: auth token memory leak",
            &vec_auth,
        )
        .unwrap();
    store
        .save_embedding(
            "C:\\repo\\app",
            "2222bbbb",
            "style: update theme palette",
            &vec_ui,
        )
        .unwrap();

    let cached = store.load_repo_embeddings("C:\\repo\\app").unwrap();
    assert_eq!(cached.len(), 2);

    let hits = VectorSearchEngine::rank_matches(&[0.95_f32, 0.05_f32, 0.0_f32], &cached, 5, 0.50);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].commit_oid, "1111aaaa");
    assert!(hits[0].similarity_score > 0.99);

    let report = SecurityAuditReport {
        scan_id: "scan-1".into(),
        repo_path: "C:\\repo\\app".into(),
        total_findings: 1,
        critical_count: 1,
        high_count: 0,
        medium_count: 0,
        low_count: 0,
        findings: vec![VulnerabilityFinding {
            id: "f1".into(),
            title: "PAT Leak".into(),
            description: "Hardcoded secret".into(),
            severity: VulnerabilitySeverity::Critical,
            cwe_id: Some("CWE-798".into()),
            owasp_category: None,
            file_path: "src/lib.rs".into(),
            line_start: 1,
            line_end: 1,
            vulnerable_snippet: "ghp_***".into(),
            suggested_fix: None,
        }],
        scan_duration_ms: 4,
        passed: false,
    };
    store.record_security_scan(&report).unwrap();
}
