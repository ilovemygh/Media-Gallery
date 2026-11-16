use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ALPN_CTRL: &[u8] = b"mg/ctrl/1";
pub const ALPN_INDEX: &[u8] = b"mg/index/1";
pub const ALPN_FILES: &[u8] = b"mg/files/1";

#[derive(Debug, Error)]
pub enum NetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Index(#[from] core_index::IndexError),
    #[error("platform error: {0}")]
    Platform(#[from] core_platform::PlatformError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("pairing failed: {0}")]
    Pairing(String),
    #[error("peer is not paired: {0}")]
    UnpairedPeer(String),
    #[error("peer is offline or unreachable: {0}")]
    PeerOffline(String),
    #[error("BLAKE3 hash mismatch for {file_name}: expected {expected}, got {actual}")]
    HashMismatch {
        file_name: String,
        expected: String,
        actual: String,
    },
    #[error("QUIC transport error: {0}")]
    Quic(String),
    #[error("no available transport path for peer: {0}")]
    NoPath(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CtrlMessage {
    Ping {
        from_device: String,
    },
    PairRequest {
        from_device: String,
        pin: String,
    },
    TransferApprovalRequest {
        transfer_id: String,
        file_name: String,
        size: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtrlResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunkFrame {
    pub transfer_id: String,
    pub media_id: String,
    pub file_name: String,
    pub chunk_index: u64,
    pub total_chunks: u64,
    pub offset: u64,
    pub bytes_total: u64,
    pub expected_blake3: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunkAck {
    pub transfer_id: String,
    pub chunk_index: u64,
    pub ok: bool,
    pub error: Option<String>,
}
