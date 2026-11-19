use gitupload_core::errors::GhCliError;
use gitupload_ghcli::{GhInstaller, GhLifecycleManager};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn build_mock_gh_zip(payload: &[u8]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("gh_2.63.0_windows_amd64/bin/gh.exe", opts)
            .unwrap();
        zip.write_all(payload).unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

#[tokio::test]
async fn test_ghcli_sandbox_installer_and_checksum_ac_p3_01() {
    let server = MockServer::start().await;
    let fake_exe = b"MZ-MOCK-GH-CLI-BINARY-2.63.0";
    let zip_bytes = build_mock_gh_zip(fake_exe);

    let mut hasher = Sha256::new();
    hasher.update(&zip_bytes);
    let sha256_hex = hex::encode(hasher.finalize());

    let asset_name = "gh_2.63.0_windows_amd64.zip";
    let checksum_name = "gh_2.63.0_checksums.txt";
    let checksums_body = format!("{sha256_hex}  {asset_name}\n0000deadbeef  other.zip\n");

    let release_json = serde_json::json!({
        "tag_name": "v2.63.0",
        "assets": [
            {
                "name": asset_name,
                "browser_download_url": format!("{}/download/{asset_name}", server.uri())
            },
            {
                "name": checksum_name,
                "browser_download_url": format!("{}/download/{checksum_name}", server.uri())
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/repos/cli/cli/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_json))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/download/{checksum_name}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(checksums_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/download/{asset_name}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes.clone()))
        .mount(&server)
        .await;

    let sandbox = tempdir().unwrap();
    let extracted_path =
        GhInstaller::download_and_extract_from_api(&server.uri(), "cli/cli", sandbox.path())
            .await
            .unwrap();

    assert!(extracted_path.is_file());
    assert!(extracted_path.starts_with(sandbox.path()));
    assert_eq!(std::fs::read(&extracted_path).unwrap(), fake_exe);

    // Sandbox resolver prioritizes local sandbox binary over global PATH.
    let manager = GhLifecycleManager::with_sandbox_dir(sandbox.path().to_path_buf());
    let resolved = manager.resolve_binary_path().unwrap();
    assert_eq!(resolved, extracted_path);

    // Tampered archive fails SHA256 verification.
    let mismatch = GhLifecycleManager::verify_checksum(
        &zip_bytes,
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    )
    .unwrap_err();
    assert!(matches!(mismatch, GhCliError::ChecksumMismatch { .. }));
}
