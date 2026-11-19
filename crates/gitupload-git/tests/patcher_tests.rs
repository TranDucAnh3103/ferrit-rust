use gitupload_core::models::SecurityPatchProposal;
use gitupload_git::AutomatedPatcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_patcher_applies_exact_remediation_and_blocks_traversal() {
    let dir = tempdir().expect("Failed to create tempdir");
    let file_path = dir.path().join("main.rs");

    let initial_content =
        "fn main() {\n    let query = \"SELECT * FROM users WHERE id = \" + id;\n}\n";
    fs::write(&file_path, initial_content).expect("Failed to write initial file");

    let patch = SecurityPatchProposal {
        patch_description: "Use parameterized query".into(),
        original_code: "let query = \"SELECT * FROM users WHERE id = \" + id;".into(),
        replacement_code:
            "let query = sqlx::query(\"SELECT * FROM users WHERE id = $1\").bind(id);".into(),
        diff_unified: String::new(),
    };

    AutomatedPatcher::apply_patch(dir.path(), "main.rs", &patch)
        .expect("Patch application must succeed");

    let updated = fs::read_to_string(&file_path).expect("Failed to read updated file");
    assert!(
        updated.contains("sqlx::query"),
        "Patched file must contain replacement code"
    );
    assert!(
        !updated.contains("\"SELECT * FROM users WHERE id = \" + id"),
        "Vulnerable snippet must be replaced"
    );

    let traversal_err = AutomatedPatcher::apply_patch(dir.path(), "../../etc/passwd", &patch)
        .expect_err("Path traversal outside repository must be blocked");
    assert!(traversal_err.to_string().contains("outside repository"));
}
