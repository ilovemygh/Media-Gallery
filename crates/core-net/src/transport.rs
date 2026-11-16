use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use core_index::{AppSettings, DeviceRecord, IndexDelta, TransportKind};
use iroh::{endpoint::presets, Endpoint, EndpointAddr, RelayMode, RelayUrl, SecretKey};
use tokio::sync::{mpsc, Mutex};

use crate::alpn::{
    CtrlMessage, CtrlResponse, FileChunkAck, FileChunkFrame, NetError, ALPN_CTRL, ALPN_FILES,
    ALPN_INDEX,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type ChunkHandler = Arc<dyn Fn(FileChunkFrame) -> Result<(), NetError> + Send + Sync>;
pub type DeltaHandler = Arc<dyn Fn(IndexDelta) -> Result<(), NetError> + Send + Sync>;

/// Modular transport trait implemented by iroh QUIC (LAN / Hole-Punched / Relay),
/// USB wired tunnels, and Bluetooth fallback channels (PRD D4–D6, D9, Section 11.2).
pub trait TransportChannel: Send + Sync {
    fn kind(&self) -> TransportKind;
    fn display_name(&self) -> &'static str;
    fn is_slow_fallback(&self) -> bool;
    fn slow_warning(&self) -> Option<&'static str>;

    fn is_available(&self, peer: &DeviceRecord) -> bool;

    fn send_chunk<'a>(
        &'a self,
        peer: &'a DeviceRecord,
        frame: &'a FileChunkFrame,
    ) -> BoxFuture<'a, Result<FileChunkAck, NetError>>;
}

/// Real `iroh` QUIC node multiplexing `mg/ctrl/1`, `mg/index/1`, and `mg/files/1` ALPNs.
pub struct IrohQuicNode {
    pub endpoint: Endpoint,
    shutdown_tx: mpsc::Sender<()>,
}

impl IrohQuicNode {
    /// Binds a real iroh QUIC endpoint.
    /// When `use_default_relay` is `false`, uses `presets::Minimal` with `RelayMode::Disabled`
    /// for pure direct LAN/loopback connections; when `true`, enables `presets::N0` with
    /// hole-punching and relay fallback (or optional `custom_relay_url`).
    pub async fn bind(
        use_default_relay: bool,
        custom_relay_url: Option<&str>,
        on_chunk: ChunkHandler,
        on_delta: DeltaHandler,
    ) -> Result<Self, NetError> {
        let key = SecretKey::generate();
        let mut builder = if use_default_relay {
            let mut b = Endpoint::builder(presets::N0);
            if let Some(url_str) = custom_relay_url {
                let url: RelayUrl = url_str
                    .parse()
                    .map_err(|e| NetError::Quic(format!("invalid relay url: {e}")))?;
                b = b.relay_mode(RelayMode::Custom(url.into()));
            } else {
                b = b.relay_mode(RelayMode::Default);
            }
            b
        } else {
            Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled)
        };

        builder = builder.secret_key(key).alpns(vec![
            ALPN_CTRL.to_vec(),
            ALPN_INDEX.to_vec(),
            ALPN_FILES.to_vec(),
        ]);

        let endpoint = builder
            .bind()
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let ep_clone = endpoint.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    incoming_opt = ep_clone.accept() => {
                        let Some(incoming) = incoming_opt else { break; };
                        let chunk_cb = Arc::clone(&on_chunk);
                        let delta_cb = Arc::clone(&on_delta);
                        tokio::spawn(async move {
                            let Ok(conn) = incoming.await else { return; };
                            let alpn = conn.alpn().to_vec();
                            while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                                let Ok(bytes) = recv.read_to_end(16 * 1024 * 1024).await else {
                                    break;
                                };
                                if alpn == ALPN_FILES {
                                    if let Ok(frame) = serde_json::from_slice::<FileChunkFrame>(&bytes) {
                                        let res = chunk_cb(frame.clone());
                                        let ack = match res {
                                            Ok(()) => FileChunkAck {
                                                transfer_id: frame.transfer_id,
                                                chunk_index: frame.chunk_index,
                                                ok: true,
                                                error: None,
                                            },
                                            Err(err) => FileChunkAck {
                                                transfer_id: frame.transfer_id,
                                                chunk_index: frame.chunk_index,
                                                ok: false,
                                                error: Some(err.to_string()),
                                            },
                                        };
                                        if let Ok(ack_bytes) = serde_json::to_vec(&ack) {
                                            let _ = send.write_all(&ack_bytes).await;
                                        }
                                    }
                                } else if alpn == ALPN_INDEX {
                                    let resp = match serde_json::from_slice::<IndexDelta>(&bytes) {
                                        Ok(delta) => match delta_cb(delta) {
                                            Ok(()) => CtrlResponse { ok: true, message: "delta applied".into() },
                                            Err(e) => CtrlResponse { ok: false, message: e.to_string() },
                                        },
                                        Err(e) => CtrlResponse { ok: false, message: e.to_string() },
                                    };
                                    if let Ok(resp_bytes) = serde_json::to_vec(&resp) {
                                        let _ = send.write_all(&resp_bytes).await;
                                    }
                                } else if alpn == ALPN_CTRL {
                                    let msg: Result<CtrlMessage, _> = serde_json::from_slice(&bytes);
                                    let resp = match msg {
                                        Ok(CtrlMessage::Ping { from_device }) => CtrlResponse {
                                            ok: true,
                                            message: format!("pong:{from_device}"),
                                        },
                                        Ok(_) => CtrlResponse {
                                            ok: true,
                                            message: "accepted".into(),
                                        },
                                        Err(e) => CtrlResponse {
                                            ok: false,
                                            message: e.to_string(),
                                        },
                                    };
                                    if let Ok(resp_bytes) = serde_json::to_vec(&resp) {
                                        let _ = send.write_all(&resp_bytes).await;
                                    }
                                }
                                let _ = send.finish();
                            }
                        });
                    }
                }
            }
        });

        Ok(Self {
            endpoint,
            shutdown_tx,
        })
    }

    pub fn node_id_string(&self) -> String {
        self.endpoint.id().to_string()
    }

    pub fn addr_json(&self) -> Result<String, NetError> {
        Ok(serde_json::to_string(&self.endpoint.addr())?)
    }

    pub async fn send_ctrl(
        &self,
        target_addr: EndpointAddr,
        msg: &CtrlMessage,
    ) -> Result<CtrlResponse, NetError> {
        let conn = self
            .endpoint
            .connect(target_addr, ALPN_CTRL)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let payload = serde_json::to_vec(msg)?;
        send.write_all(&payload)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        send.finish().map_err(|e| NetError::Quic(e.to_string()))?;
        let reply_bytes = recv
            .read_to_end(1024 * 1024)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        Ok(serde_json::from_slice(&reply_bytes)?)
    }

    pub async fn send_index_delta(
        &self,
        target_addr: EndpointAddr,
        delta: &IndexDelta,
    ) -> Result<CtrlResponse, NetError> {
        let conn = self
            .endpoint
            .connect(target_addr, ALPN_INDEX)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let payload = serde_json::to_vec(delta)?;
        send.write_all(&payload)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        send.finish().map_err(|e| NetError::Quic(e.to_string()))?;
        let reply_bytes = recv
            .read_to_end(1024 * 1024)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        Ok(serde_json::from_slice(&reply_bytes)?)
    }

    pub async fn send_file_chunk(
        &self,
        target_addr: EndpointAddr,
        frame: &FileChunkFrame,
    ) -> Result<FileChunkAck, NetError> {
        let conn = self
            .endpoint
            .connect(target_addr, ALPN_FILES)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let payload = serde_json::to_vec(frame)?;
        send.write_all(&payload)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        send.finish().map_err(|e| NetError::Quic(e.to_string()))?;
        let reply_bytes = recv
            .read_to_end(1024 * 1024)
            .await
            .map_err(|e| NetError::Quic(e.to_string()))?;
        let ack: FileChunkAck = serde_json::from_slice(&reply_bytes)?;
        if !ack.ok {
            return Err(NetError::Quic(
                ack.error.unwrap_or_else(|| "remote chunk rejected".into()),
            ));
        }
        Ok(ack)
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(()).await;
        self.endpoint.close().await;
    }
}

/// TransportChannel implementation backed by a live `IrohQuicNode` + address book.
pub struct IrohQuicTransport {
    kind: TransportKind,
    node: Arc<IrohQuicNode>,
    peer_addrs: Mutex<HashMap<String, EndpointAddr>>,
}

impl IrohQuicTransport {
    pub fn new(kind: TransportKind, node: Arc<IrohQuicNode>) -> Self {
        Self {
            kind,
            node,
            peer_addrs: Mutex::new(HashMap::new()),
        }
    }

    pub async fn register_peer_addr(&self, device_id: &str, addr: EndpointAddr) {
        let mut guard = self.peer_addrs.lock().await;
        guard.insert(device_id.to_string(), addr);
    }
}

impl TransportChannel for IrohQuicTransport {
    fn kind(&self) -> TransportKind {
        self.kind
    }

    fn display_name(&self) -> &'static str {
        match self.kind {
            TransportKind::Wired => "Wired USB (QUIC Tunnel)",
            TransportKind::WifiLan => "Wi-Fi LAN (iroh QUIC Direct)",
            TransportKind::HolePunched => "Internet (iroh Hole-Punched QUIC)",
            TransportKind::Relay => "Relay Fallback (iroh E2E Relay)",
            TransportKind::Bluetooth => "Bluetooth",
        }
    }

    fn is_slow_fallback(&self) -> bool {
        self.kind == TransportKind::Bluetooth
    }

    fn slow_warning(&self) -> Option<&'static str> {
        if self.kind == TransportKind::Bluetooth {
            Some("Bluetooth transfer is slow (~1-2 MB/s). Recommended for small files only.")
        } else {
            None
        }
    }

    fn is_available(&self, peer: &DeviceRecord) -> bool {
        peer.is_online && peer.transports.contains(&self.kind)
    }

    fn send_chunk<'a>(
        &'a self,
        peer: &'a DeviceRecord,
        frame: &'a FileChunkFrame,
    ) -> BoxFuture<'a, Result<FileChunkAck, NetError>> {
        Box::pin(async move {
            let addr = {
                let guard = self.peer_addrs.lock().await;
                guard.get(&peer.id).cloned()
            }
            .ok_or_else(|| NetError::PeerOffline(format!("no EndpointAddr for {}", peer.id)))?;
            self.node.send_file_chunk(addr, frame).await
        })
    }
}

/// Wired USB / ADB reverse tunnel transport (PRD D4).
pub struct UsbWiredTransport {
    adb_devices: Vec<String>,
    inner: Option<Arc<dyn TransportChannel>>,
}

impl UsbWiredTransport {
    pub fn new(adb_devices: Vec<String>, inner: Option<Arc<dyn TransportChannel>>) -> Self {
        Self { adb_devices, inner }
    }
}

impl TransportChannel for UsbWiredTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Wired
    }

    fn display_name(&self) -> &'static str {
        "Wired USB (ADB Reverse / Tethering)"
    }

    fn is_slow_fallback(&self) -> bool {
        false
    }

    fn slow_warning(&self) -> Option<&'static str> {
        None
    }

    fn is_available(&self, peer: &DeviceRecord) -> bool {
        peer.is_online
            && peer.transports.contains(&TransportKind::Wired)
            && !self.adb_devices.is_empty()
    }

    fn send_chunk<'a>(
        &'a self,
        peer: &'a DeviceRecord,
        frame: &'a FileChunkFrame,
    ) -> BoxFuture<'a, Result<FileChunkAck, NetError>> {
        Box::pin(async move {
            let Some(ref inner) = self.inner else {
                return Err(NetError::NoPath("wired tunnel backend not attached".into()));
            };
            inner.send_chunk(peer, frame).await
        })
    }
}

/// Bluetooth fallback transport (PRD D5). Always warns user about slow throughput.
pub struct BluetoothFallbackTransport {
    sink: Option<ChunkHandler>,
}

impl BluetoothFallbackTransport {
    pub fn new(sink: Option<ChunkHandler>) -> Self {
        Self { sink }
    }
}

impl TransportChannel for BluetoothFallbackTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Bluetooth
    }

    fn display_name(&self) -> &'static str {
        "Bluetooth (Slow Fallback)"
    }

    fn is_slow_fallback(&self) -> bool {
        true
    }

    fn slow_warning(&self) -> Option<&'static str> {
        Some("Bluetooth transfer is slow (~1-2 MB/s). Use USB or Wi-Fi/iroh when available.")
    }

    fn is_available(&self, peer: &DeviceRecord) -> bool {
        peer.is_online && peer.transports.contains(&TransportKind::Bluetooth)
    }

    fn send_chunk<'a>(
        &'a self,
        _peer: &'a DeviceRecord,
        frame: &'a FileChunkFrame,
    ) -> BoxFuture<'a, Result<FileChunkAck, NetError>> {
        Box::pin(async move {
            if let Some(ref sink) = self.sink {
                sink(frame.clone())?;
            }
            Ok(FileChunkAck {
                transfer_id: frame.transfer_id.clone(),
                chunk_index: frame.chunk_index,
                ok: true,
                error: None,
            })
        })
    }
}

/// Test helper transport that intentionally drops connection after `fail_after_chunks`
/// to verify chunked interruption, packet-loss recovery, and resumable BLAKE3 transfers.
pub struct SimulatedFaultyTransport {
    pub inner: Arc<dyn TransportChannel>,
    pub fail_after_chunks: u64,
    sent_count: AtomicU64,
}

impl SimulatedFaultyTransport {
    pub fn new(inner: Arc<dyn TransportChannel>, fail_after_chunks: u64) -> Self {
        Self {
            inner,
            fail_after_chunks,
            sent_count: AtomicU64::new(0),
        }
    }
}

impl TransportChannel for SimulatedFaultyTransport {
    fn kind(&self) -> TransportKind {
        self.inner.kind()
    }

    fn display_name(&self) -> &'static str {
        self.inner.display_name()
    }

    fn is_slow_fallback(&self) -> bool {
        self.inner.is_slow_fallback()
    }

    fn slow_warning(&self) -> Option<&'static str> {
        self.inner.slow_warning()
    }

    fn is_available(&self, peer: &DeviceRecord) -> bool {
        self.inner.is_available(peer)
    }

    fn send_chunk<'a>(
        &'a self,
        peer: &'a DeviceRecord,
        frame: &'a FileChunkFrame,
    ) -> BoxFuture<'a, Result<FileChunkAck, NetError>> {
        Box::pin(async move {
            let count = self.sent_count.fetch_add(1, Ordering::Relaxed);
            if count >= self.fail_after_chunks {
                return Err(NetError::Quic(
                    "simulated packet loss / connection drop".into(),
                ));
            }
            self.inner.send_chunk(peer, frame).await
        })
    }
}

/// Automatic path priority selector (PRD D6 & D9):
/// Wired > Wi-Fi LAN > Hole-Punched Internet > Relay > Bluetooth (with manual override).
#[derive(Default)]
pub struct PathSelector {
    channels: HashMap<TransportKind, Arc<dyn TransportChannel>>,
}

pub struct SelectedPath {
    pub channel: Arc<dyn TransportChannel>,
    pub warning: Option<&'static str>,
}

impl PathSelector {
    pub fn register(&mut self, channel: Arc<dyn TransportChannel>) {
        self.channels.insert(channel.kind(), channel);
    }

    pub fn select_best(
        &self,
        peer: &DeviceRecord,
        settings: &AppSettings,
    ) -> Result<SelectedPath, NetError> {
        if let Some(override_kind) = settings.manual_transport_override {
            let ch = self.channels.get(&override_kind).ok_or_else(|| {
                NetError::NoPath(format!(
                    "override transport {override_kind:?} not registered"
                ))
            })?;
            if ch.is_available(peer) {
                return Ok(SelectedPath {
                    warning: ch.slow_warning(),
                    channel: Arc::clone(ch),
                });
            }
            return Err(NetError::NoPath(format!(
                "manually selected transport {override_kind:?} unavailable for {}",
                peer.name
            )));
        }

        let priority = if settings.transport_priority.is_empty() {
            vec![
                TransportKind::Wired,
                TransportKind::WifiLan,
                TransportKind::HolePunched,
                TransportKind::Relay,
                TransportKind::Bluetooth,
            ]
        } else {
            settings.transport_priority.clone()
        };

        for kind in priority {
            if let Some(ch) = self.channels.get(&kind) {
                if ch.is_available(peer) {
                    return Ok(SelectedPath {
                        warning: ch.slow_warning(),
                        channel: Arc::clone(ch),
                    });
                }
            }
        }

        Err(NetError::NoPath(peer.name.clone()))
    }
}
