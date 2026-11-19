//! Pure-Rust SSH Agent wire protocol codec (`draft-miller-ssh-agent`).

use crate::ssh::keygen::push_ssh_string;
use gitupload_core::errors::{GitUploadError, GitUploadResult};

/// SSH Agent wire protocol message numbers.
pub const SSH_AGENT_FAILURE: u8 = 5;
pub const SSH_AGENT_SUCCESS: u8 = 6;
pub const SSH2_AGENTC_REQUEST_IDENTITIES: u8 = 11;
pub const SSH2_AGENT_IDENTITIES_ANSWER: u8 = 12;
pub const SSH2_AGENTC_SIGN_REQUEST: u8 = 13;
pub const SSH2_AGENT_SIGN_RESPONSE: u8 = 14;

/// Decoded SSH Agent client request message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshAgentRequest {
    /// Request list of currently unlocked public keys (`11`).
    RequestIdentities,
    /// Sign challenge payload with key matching `key_blob` (`13`).
    SignRequest {
        /// OpenSSH public key wire blob.
        key_blob: Vec<u8>,
        /// Challenge payload to sign.
        data: Vec<u8>,
        /// RFC 8332 signature algorithm flags.
        flags: u32,
    },
    /// Any unsupported or unhandled client opcode.
    Unsupported(u8),
}

struct WireReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> WireReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_u8(&mut self) -> GitUploadResult<u8> {
        if self.pos >= self.buf.len() {
            return Err(GitUploadError::Vault(
                "Unexpected EOF reading SSH agent byte".to_string(),
            ));
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u32(&mut self) -> GitUploadResult<u32> {
        if self.pos + 4 > self.buf.len() {
            return Err(GitUploadError::Vault(
                "Unexpected EOF reading SSH agent uint32".to_string(),
            ));
        }
        let val = u32::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(val)
    }

    fn read_string(&mut self) -> GitUploadResult<Vec<u8>> {
        let len = self.read_u32()? as usize;
        if len > 16 * 1024 * 1024 || self.pos + len > self.buf.len() {
            return Err(GitUploadError::Vault(
                "Invalid or truncated SSH agent string field".to_string(),
            ));
        }
        let slice = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice.to_vec())
    }
}

/// Parses a single SSH Agent request frame payload (without the 4-byte length prefix).
pub fn parse_request(payload: &[u8]) -> GitUploadResult<SshAgentRequest> {
    let mut reader = WireReader::new(payload);
    let opcode = reader.read_u8()?;
    match opcode {
        SSH2_AGENTC_REQUEST_IDENTITIES => Ok(SshAgentRequest::RequestIdentities),
        SSH2_AGENTC_SIGN_REQUEST => {
            let key_blob = reader.read_string()?;
            let data = reader.read_string()?;
            let flags = reader.read_u32()?;
            Ok(SshAgentRequest::SignRequest {
                key_blob,
                data,
                flags,
            })
        }
        other => Ok(SshAgentRequest::Unsupported(other)),
    }
}

/// Encodes `SSH2_AGENT_IDENTITIES_ANSWER` (`12`) payload (without outer 4-byte frame length).
pub fn encode_identities_answer(identities: &[(Vec<u8>, String)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.push(SSH2_AGENT_IDENTITIES_ANSWER);
    out.extend_from_slice(&(identities.len() as u32).to_be_bytes());
    for (blob, comment) in identities {
        push_ssh_string(&mut out, blob);
        push_ssh_string(&mut out, comment.as_bytes());
    }
    out
}

/// Encodes `SSH2_AGENT_SIGN_RESPONSE` (`14`) payload (without outer 4-byte frame length).
pub fn encode_sign_response(signature_blob: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + signature_blob.len());
    out.push(SSH2_AGENT_SIGN_RESPONSE);
    push_ssh_string(&mut out, signature_blob);
    out
}

/// Encodes `SSH_AGENT_FAILURE` (`5`) payload.
pub fn encode_failure() -> Vec<u8> {
    vec![SSH_AGENT_FAILURE]
}

/// Prepends the 4-byte big-endian frame length header expected by the SSH agent wire protocol.
pub fn frame_message(payload: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(4 + payload.len());
    framed.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    framed.extend_from_slice(payload);
    framed
}
