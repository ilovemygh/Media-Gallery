//! Key-dialed iroh QUIC networking, QR/PIN pairing, multi-path transport selection,
//! and resumable BLAKE3-verified chunked transfers.

pub mod alpn;
pub mod pairing;
pub mod transfer;
pub mod transport;

pub use alpn::{
    CtrlMessage, CtrlResponse, FileChunkAck, FileChunkFrame, NetError, ALPN_CTRL, ALPN_FILES,
    ALPN_INDEX,
};
pub use iroh::EndpointAddr;
pub use pairing::{PairingManager, PairingSession, QrPairingData};
pub use transfer::{TransferEngine, TransferReceiver, DEFAULT_CHUNK_SIZE};
pub use transport::{
    BluetoothFallbackTransport, IrohQuicNode, IrohQuicTransport, PathSelector, SelectedPath,
    SimulatedFaultyTransport, TransportChannel, UsbWiredTransport,
};
