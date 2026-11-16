use std::sync::{Arc, Mutex};

use core_index::{now_ms, DeviceRecord, DeviceType, MediaIndex, TransportKind};
use core_platform::SecureStore;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::alpn::NetError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingSession {
    pub pin: String,
    pub token: String,
    pub qr_payload: String,
    pub device_id: String,
    pub device_name: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPairingData {
    pub device_id: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub endpoint_addr_json: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub public_key: String,
    pub pin_token: String,
    pub expires_at: i64,
}

pub struct PairingManager {
    index: MediaIndex,
    secure_store: Arc<dyn SecureStore>,
    active_session: Mutex<Option<PairingSession>>,
}

impl PairingManager {
    pub fn new(index: MediaIndex, secure_store: Arc<dyn SecureStore>) -> Self {
        Self {
            index,
            secure_store,
            active_session: Mutex::new(None),
        }
    }

    pub fn create_session(
        &self,
        self_device: &DeviceRecord,
        endpoint_addr_json: &str,
        ttl_ms: i64,
    ) -> Result<PairingSession, NetError> {
        let pin_num: u32 = rand::rng().random_range(0..1_000_000);
        let pin = format!("{pin_num:06}");
        let token = Uuid::new_v4().simple().to_string();
        let expires_at = now_ms() + ttl_ms;

        let qr_data = QrPairingData {
            device_id: self_device.id.clone(),
            device_name: self_device.name.clone(),
            device_type: self_device.device_type,
            endpoint_addr_json: endpoint_addr_json.to_string(),
            host: self_device.host.clone(),
            port: self_device.port,
            public_key: self_device.public_key.clone(),
            pin_token: format!("{pin}:{token}"),
            expires_at,
        };
        let qr_payload = serde_json::to_string(&qr_data)?;
        let session = PairingSession {
            pin,
            token,
            qr_payload,
            device_id: self_device.id.clone(),
            device_name: self_device.name.clone(),
            expires_at,
        };

        *self
            .active_session
            .lock()
            .map_err(|_| NetError::Pairing("session lock poisoned".into()))? =
            Some(session.clone());
        Ok(session)
    }

    pub fn pair_with_pin(&self, pin: &str, remote: DeviceRecord) -> Result<DeviceRecord, NetError> {
        let session = {
            self.active_session
                .lock()
                .map_err(|_| NetError::Pairing("session lock poisoned".into()))?
                .clone()
        };
        let Some(sess) = session else {
            return Err(NetError::Pairing("no active pairing session".into()));
        };
        if now_ms() > sess.expires_at {
            return Err(NetError::Pairing("pairing PIN has expired".into()));
        }
        if pin.trim() != sess.pin {
            return Err(NetError::Pairing("invalid 6-digit pairing PIN".into()));
        }
        self.finalize_pairing(remote, &sess.token)
    }

    pub fn pair_with_qr(
        &self,
        qr_payload_json: &str,
        remote_override: Option<DeviceRecord>,
    ) -> Result<DeviceRecord, NetError> {
        let data: QrPairingData = serde_json::from_str(qr_payload_json)
            .map_err(|e| NetError::Pairing(format!("invalid QR payload: {e}")))?;
        if now_ms() > data.expires_at {
            return Err(NetError::Pairing("pairing QR payload has expired".into()));
        }
        let peer = remote_override.unwrap_or(DeviceRecord {
            id: data.device_id,
            name: data.device_name,
            device_type: data.device_type,
            public_key: data.public_key,
            last_seen: now_ms(),
            transports: vec![TransportKind::WifiLan, TransportKind::Relay],
            host: data.host,
            port: data.port,
            is_self: false,
            is_online: true,
            paired_at: Some(now_ms()),
        });
        let secret = format!("{}::{}", data.pin_token, data.endpoint_addr_json);
        self.finalize_pairing(peer, &secret)
    }

    fn finalize_pairing(
        &self,
        mut remote: DeviceRecord,
        shared_secret: &str,
    ) -> Result<DeviceRecord, NetError> {
        let ts = now_ms();
        remote.is_self = false;
        remote.is_online = true;
        remote.last_seen = ts;
        remote.paired_at = Some(ts);
        self.index.upsert_device(&remote)?;
        self.secure_store
            .set_secret(&format!("peer:{}:auth", remote.id), shared_secret)?;
        self.secure_store
            .set_secret(&format!("peer:{}:pubkey", remote.id), &remote.public_key)?;
        if let Ok(mut guard) = self.active_session.lock() {
            *guard = None;
        }
        Ok(remote)
    }

    pub fn is_paired(&self, device_id: &str) -> Result<bool, NetError> {
        let dev = self.index.get_device(device_id)?;
        Ok(dev.is_some_and(|d| d.paired_at.is_some() && !d.public_key.is_empty()))
    }

    pub fn requires_confirmation(&self, sender_id: &str) -> Result<bool, NetError> {
        Ok(!self.is_paired(sender_id)?)
    }

    pub fn get_pinned_public_key(&self, device_id: &str) -> Result<Option<String>, NetError> {
        if let Some(pk) = self
            .secure_store
            .get_secret(&format!("peer:{device_id}:pubkey"))?
        {
            return Ok(Some(pk));
        }
        Ok(self.index.get_device(device_id)?.map(|d| d.public_key))
    }

    pub fn forget_device(&self, device_id: &str) -> Result<(), NetError> {
        self.index.forget_device(device_id)?;
        self.secure_store
            .delete_secret(&format!("peer:{device_id}:auth"))?;
        self.secure_store
            .delete_secret(&format!("peer:{device_id}:pubkey"))?;
        Ok(())
    }
}
