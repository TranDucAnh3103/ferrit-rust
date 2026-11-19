//! Pure-Rust SSH cryptography, OS Keyring private-key vault, and Windows Named Pipe SSH Agent.

pub mod agent;
pub mod keygen;
pub mod protocol;
pub mod vault;

pub use agent::{send_named_pipe_request, NativeSshAgent, DEFAULT_WINDOWS_SSH_AGENT_PIPE};
pub use keygen::{
    compute_fingerprint_sha256, decode_openssh_pubkey_blob, generate_ssh_keypair,
    GeneratedSshKeypair,
};
pub use protocol::{
    encode_identities_answer, encode_sign_response, frame_message, parse_request, SshAgentRequest,
    SSH2_AGENTC_REQUEST_IDENTITIES, SSH2_AGENTC_SIGN_REQUEST, SSH2_AGENT_IDENTITIES_ANSWER,
    SSH2_AGENT_SIGN_RESPONSE, SSH_AGENT_FAILURE, SSH_AGENT_SUCCESS,
};
pub use vault::SshVault;
