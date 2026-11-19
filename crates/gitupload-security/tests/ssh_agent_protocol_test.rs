use ed25519_dalek::{Signature, VerifyingKey};
use gitupload_core::models::SshKeyAlgorithm;
use gitupload_security::ssh::protocol::{
    SSH2_AGENTC_REQUEST_IDENTITIES, SSH2_AGENTC_SIGN_REQUEST, SSH2_AGENT_IDENTITIES_ANSWER,
    SSH2_AGENT_SIGN_RESPONSE,
};
use gitupload_security::{generate_ssh_keypair, KeyringManager, NativeSshAgent, SshVault};
use std::sync::Arc;
use uuid::Uuid;

fn read_u32(slice: &[u8], pos: &mut usize) -> u32 {
    let val = u32::from_be_bytes([
        slice[*pos],
        slice[*pos + 1],
        slice[*pos + 2],
        slice[*pos + 3],
    ]);
    *pos += 4;
    val
}

fn read_string<'a>(slice: &'a [u8], pos: &mut usize) -> &'a [u8] {
    let len = read_u32(slice, pos) as usize;
    let out = &slice[*pos..*pos + len];
    *pos += len;
    out
}

#[tokio::test]
async fn test_ssh_agent_named_pipe_protocol_ed25519_roundtrip() {
    let keyring = Arc::new(KeyringManager::new_in_memory());
    let vault = Arc::new(SshVault::with_keyring(keyring));

    let generated = generate_ssh_keypair("Sprint2 Test Ed25519", SshKeyAlgorithm::Ed25519).unwrap();
    assert!(generated.descriptor.public_key.starts_with("ssh-ed25519 "));
    assert!(generated
        .descriptor
        .fingerprint_sha256
        .starts_with("SHA256:"));
    vault.store_generated_keypair(&generated).unwrap();

    // Test chunked RSA-like large payload storage in vault without exhausting Windows blob limits
    let big_payload = "X".repeat(2500);
    vault
        .store_private_material("large-rsa-test", &big_payload)
        .unwrap();
    let loaded_big = vault.load_private_key("large-rsa-test").unwrap().unwrap();
    use secrecy::ExposeSecret;
    assert_eq!(loaded_big.expose_secret().len(), 2500);

    let pipe_name = format!(r"\\.\pipe\gitupload-ssh-test-{}", Uuid::new_v4());
    let agent = NativeSshAgent::new(&pipe_name, Arc::clone(&vault));
    agent
        .start(std::slice::from_ref(&generated.descriptor))
        .await
        .unwrap();
    assert!(agent.is_running());
    assert_eq!(agent.loaded_key_count(), 1);

    // 1. Request identities over live Windows Named Pipe
    let id_resp = gitupload_security::ssh::send_named_pipe_request(
        &pipe_name,
        &[SSH2_AGENTC_REQUEST_IDENTITIES],
    )
    .await
    .unwrap();
    assert_eq!(id_resp[0], SSH2_AGENT_IDENTITIES_ANSWER);
    let mut pos = 1usize;
    let count = read_u32(&id_resp, &mut pos);
    assert_eq!(count, 1);
    let returned_blob = read_string(&id_resp, &mut pos);
    assert_eq!(returned_blob, generated.public_key_blob.as_slice());

    // 2. Request signature over live Windows Named Pipe
    let challenge = b"GitUpload SSH challenge 2026-sprint-2";
    let mut sign_req = Vec::new();
    sign_req.push(SSH2_AGENTC_SIGN_REQUEST);
    sign_req.extend_from_slice(&(generated.public_key_blob.len() as u32).to_be_bytes());
    sign_req.extend_from_slice(&generated.public_key_blob);
    sign_req.extend_from_slice(&(challenge.len() as u32).to_be_bytes());
    sign_req.extend_from_slice(challenge);
    sign_req.extend_from_slice(&0u32.to_be_bytes());

    let sign_resp = gitupload_security::ssh::send_named_pipe_request(&pipe_name, &sign_req)
        .await
        .unwrap();
    assert_eq!(sign_resp[0], SSH2_AGENT_SIGN_RESPONSE);

    let mut spos = 1usize;
    let outer_sig_blob = read_string(&sign_resp, &mut spos);
    let mut inner_pos = 0usize;
    let sig_alg = read_string(outer_sig_blob, &mut inner_pos);
    let sig_raw = read_string(outer_sig_blob, &mut inner_pos);
    assert_eq!(sig_alg, b"ssh-ed25519");

    // Extract raw 32-byte Ed25519 public key from generated.public_key_blob and cryptographically verify
    let mut pk_pos = 0usize;
    let _pk_alg = read_string(&generated.public_key_blob, &mut pk_pos);
    let pk_raw = read_string(&generated.public_key_blob, &mut pk_pos);
    let verifying_key = VerifyingKey::from_bytes(pk_raw.try_into().unwrap()).unwrap();
    let signature = Signature::from_bytes(sig_raw.try_into().unwrap());
    verifying_key.verify_strict(challenge, &signature).unwrap();

    // 3. Stop wipes memory
    agent.stop().await;
    assert!(!agent.is_running());
    assert_eq!(agent.loaded_key_count(), 0);
}
