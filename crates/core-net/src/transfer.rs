use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use core_index::{
    compute_file_blake3, now_ms, DeviceRecord, MediaIndex, MediaRecord, TransferRecord,
    TransferStatus,
};
use uuid::Uuid;

use crate::alpn::{FileChunkFrame, NetError};
use crate::pairing::PairingManager;
use crate::transport::TransportChannel;

pub const DEFAULT_CHUNK_SIZE: u64 = 256 * 1024;

/// Receiver-side offset-safe file assembler with BLAKE3 integrity verification.
pub struct TransferReceiver {
    destination_dir: PathBuf,
    completed: Mutex<HashMap<String, PathBuf>>,
}

impl TransferReceiver {
    pub fn new(destination_dir: impl Into<PathBuf>) -> Self {
        Self {
            destination_dir: destination_dir.into(),
            completed: Mutex::new(HashMap::new()),
        }
    }

    pub fn part_file_path(&self, transfer_id: &str, file_name: &str) -> PathBuf {
        let base = Path::new(file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file.bin".to_string());
        self.destination_dir
            .join(format!(".part-{transfer_id}-{base}"))
    }

    pub fn final_file_path(&self, file_name: &str) -> PathBuf {
        let base = Path::new(file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file.bin".to_string());
        self.destination_dir.join(base)
    }

    pub fn completed_path(&self, transfer_id: &str) -> Option<PathBuf> {
        self.completed
            .lock()
            .ok()
            .and_then(|map| map.get(transfer_id).cloned())
    }

    pub fn handle_chunk(&self, frame: &FileChunkFrame) -> Result<Option<PathBuf>, NetError> {
        std::fs::create_dir_all(&self.destination_dir)?;
        let part_path = self.part_file_path(&frame.transfer_id, &frame.file_name);

        {
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&part_path)?;
            if !frame.data.is_empty() {
                file.seek(SeekFrom::Start(frame.offset))?;
                file.write_all(&frame.data)?;
                file.flush()?;
            }
        }

        let part_size = std::fs::metadata(&part_path)?.len();
        if part_size >= frame.bytes_total && frame.chunk_index + 1 >= frame.total_chunks {
            let actual_blake3 = compute_file_blake3(&part_path)?;
            if actual_blake3 != frame.expected_blake3 {
                return Err(NetError::HashMismatch {
                    file_name: frame.file_name.clone(),
                    expected: frame.expected_blake3.clone(),
                    actual: actual_blake3,
                });
            }
            let final_path = self.final_file_path(&frame.file_name);
            if final_path.exists() {
                let _ = std::fs::remove_file(&final_path);
            }
            std::fs::rename(&part_path, &final_path)?;
            if let Ok(mut map) = self.completed.lock() {
                map.insert(frame.transfer_id.clone(), final_path.clone());
            }
            return Ok(Some(final_path));
        }

        Ok(None)
    }
}

/// Chunked, resumable, BLAKE3-verified transfer orchestrator (PRD D7).
pub struct TransferEngine {
    index: MediaIndex,
    pairing: Option<std::sync::Arc<PairingManager>>,
    control_flags: Mutex<HashMap<String, TransferStatus>>,
}

impl TransferEngine {
    pub fn new(index: MediaIndex, pairing: Option<std::sync::Arc<PairingManager>>) -> Self {
        Self {
            index,
            pairing,
            control_flags: Mutex::new(HashMap::new()),
        }
    }

    pub fn pause_transfer(&self, transfer_id: &str) -> Result<(), NetError> {
        if let Ok(mut map) = self.control_flags.lock() {
            map.insert(transfer_id.to_string(), TransferStatus::Paused);
        }
        if let Some(mut tr) = self.index.get_transfer(transfer_id)? {
            tr.status = TransferStatus::Paused;
            tr.updated_at = now_ms();
            self.index.upsert_transfer(&tr)?;
        }
        Ok(())
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<(), NetError> {
        if let Ok(mut map) = self.control_flags.lock() {
            map.insert(transfer_id.to_string(), TransferStatus::Cancelled);
        }
        if let Some(mut tr) = self.index.get_transfer(transfer_id)? {
            tr.status = TransferStatus::Cancelled;
            tr.updated_at = now_ms();
            self.index.upsert_transfer(&tr)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_file<F>(
        &self,
        media: &MediaRecord,
        from_device_id: &str,
        to_device: &DeviceRecord,
        transport: &dyn TransportChannel,
        chunk_size: Option<u64>,
        max_chunks_this_run: Option<u64>,
        mut on_progress: F,
    ) -> Result<TransferRecord, NetError>
    where
        F: FnMut(&TransferRecord),
    {
        if let Some(ref pairing) = self.pairing {
            if !pairing.is_paired(&to_device.id)? {
                return Err(NetError::UnpairedPeer(to_device.name.clone()));
            }
        }
        if !to_device.is_online {
            return Err(NetError::PeerOffline(to_device.name.clone()));
        }

        let src_path = Path::new(&media.path);
        let total_bytes = std::fs::metadata(src_path)?.len();
        let expected_blake3 = compute_file_blake3(src_path)?;
        let chunk_sz = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(1024);

        let record = TransferRecord {
            id: Uuid::new_v4().to_string(),
            media_id: media.id.clone(),
            file_name: media.file_name.clone(),
            from_device: from_device_id.to_string(),
            to_device: to_device.id.clone(),
            status: TransferStatus::Transferring,
            bytes_done: 0,
            bytes_total: total_bytes,
            last_chunk_index: 0,
            chunk_size: chunk_sz,
            path_kind: format!("{:?}", transport.kind()),
            speed_bps: 0,
            expected_blake3,
            error: None,
            updated_at: now_ms(),
        };
        self.index.upsert_transfer(&record)?;

        self.stream_remaining(
            record,
            src_path,
            to_device,
            transport,
            max_chunks_this_run,
            &mut on_progress,
        )
        .await
    }

    pub async fn resume_transfer<F>(
        &self,
        transfer_id: &str,
        source_path: &Path,
        to_device: &DeviceRecord,
        transport: &dyn TransportChannel,
        mut on_progress: F,
    ) -> Result<TransferRecord, NetError>
    where
        F: FnMut(&TransferRecord),
    {
        let mut record = self
            .index
            .get_transfer(transfer_id)?
            .ok_or_else(|| NetError::Quic(format!("transfer not found: {transfer_id}")))?;
        if let Ok(mut map) = self.control_flags.lock() {
            map.remove(transfer_id);
        }
        record.status = TransferStatus::Transferring;
        record.path_kind = format!("{:?}", transport.kind());
        record.error = None;
        record.updated_at = now_ms();
        self.index.upsert_transfer(&record)?;

        self.stream_remaining(
            record,
            source_path,
            to_device,
            transport,
            None,
            &mut on_progress,
        )
        .await
    }

    async fn stream_remaining<F>(
        &self,
        mut current: TransferRecord,
        source_path: &Path,
        to_device: &DeviceRecord,
        transport: &dyn TransportChannel,
        max_chunks_this_run: Option<u64>,
        on_progress: &mut F,
    ) -> Result<TransferRecord, NetError>
    where
        F: FnMut(&TransferRecord),
    {
        let total_chunks = (current.bytes_total.div_ceil(current.chunk_size)).max(1);
        let mut file = File::open(source_path)?;
        let initial_bytes = current.bytes_done;
        let started = Instant::now();
        let mut sent_this_run = 0u64;

        while current.last_chunk_index < total_chunks {
            if let Ok(map) = self.control_flags.lock() {
                if let Some(flag) = map.get(&current.id) {
                    current.status = *flag;
                    current.updated_at = now_ms();
                    self.index.upsert_transfer(&current)?;
                    on_progress(&current);
                    return Ok(current);
                }
            }
            if max_chunks_this_run.is_some_and(|max| sent_this_run >= max) {
                current.status = TransferStatus::Paused;
                current.updated_at = now_ms();
                self.index.upsert_transfer(&current)?;
                on_progress(&current);
                return Ok(current);
            }

            let idx = current.last_chunk_index;
            let offset = idx * current.chunk_size;
            let remaining = current.bytes_total.saturating_sub(offset);
            let len = remaining.min(current.chunk_size) as usize;
            let mut buf = vec![0u8; len];
            if len > 0 {
                file.seek(SeekFrom::Start(offset))?;
                file.read_exact(&mut buf)?;
            }

            let frame = FileChunkFrame {
                transfer_id: current.id.clone(),
                media_id: current.media_id.clone(),
                file_name: current.file_name.clone(),
                chunk_index: idx,
                total_chunks,
                offset,
                bytes_total: current.bytes_total,
                expected_blake3: current.expected_blake3.clone(),
                data: buf,
            };

            if let Err(err) = transport.send_chunk(to_device, &frame).await {
                current.status = TransferStatus::Error;
                current.error = Some(err.to_string());
                current.updated_at = now_ms();
                self.index.upsert_transfer(&current)?;
                on_progress(&current);
                return Err(err);
            }

            sent_this_run += 1;
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let new_bytes = (offset + len as u64).min(current.bytes_total);
            current.bytes_done = new_bytes;
            current.last_chunk_index = idx + 1;
            current.speed_bps = (((new_bytes - initial_bytes) as f64) / elapsed) as u64;
            current.status = if current.last_chunk_index >= total_chunks {
                TransferStatus::Completed
            } else {
                TransferStatus::Transferring
            };
            current.updated_at = now_ms();
            self.index.upsert_transfer(&current)?;
            on_progress(&current);
        }

        Ok(current)
    }
}
