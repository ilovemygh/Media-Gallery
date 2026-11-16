use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    compute_file_blake3, AppSettings, DeviceRecord, DeviceType, MediaIndex, MediaRecord, MediaTags,
    MediaType, SyncState, TransferStatus, TransportKind,
};
use core_net::{
    BluetoothFallbackTransport, CtrlMessage, IrohQuicNode, IrohQuicTransport, NetError,
    PairingManager, PathSelector, SimulatedFaultyTransport, TransferEngine, TransferReceiver,
    UsbWiredTransport,
};
use core_platform::EncryptedMemoryStore;

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn pairing_pin_qr_allowlist_and_path_priority_selector_work() {
    let idx = MediaIndex::open_in_memory().unwrap();
    let store = Arc::new(EncryptedMemoryStore::default());
    let self_dev = DeviceRecord {
        id: "pc-1".into(),
        name: "PC".into(),
        device_type: DeviceType::Pc,
        public_key: "ed25519-pc".into(),
        last_seen: 1_700_000_000_000,
        transports: vec![
            TransportKind::Wired,
            TransportKind::WifiLan,
            TransportKind::Bluetooth,
        ],
        host: Some("127.0.0.1".into()),
        port: Some(9443),
        is_self: true,
        is_online: true,
        paired_at: None,
    };
    idx.upsert_device(&self_dev).unwrap();

    let pairing = PairingManager::new(
        idx.clone(),
        Arc::clone(&store) as Arc<dyn core_platform::SecureStore>,
    );
    let sess = pairing.create_session(&self_dev, "{}", 300_000).unwrap();
    assert_eq!(sess.pin.len(), 6);

    let wrong_pin = if sess.pin == "000000" {
        "111111"
    } else {
        "000000"
    };
    assert!(pairing
        .pair_with_pin(
            wrong_pin,
            DeviceRecord {
                id: "phone-1".into(),
                name: "Pixel".into(),
                device_type: DeviceType::Phone,
                public_key: "pk-phone".into(),
                last_seen: 0,
                transports: vec![TransportKind::WifiLan],
                host: None,
                port: None,
                is_self: false,
                is_online: true,
                paired_at: None,
            },
        )
        .is_err());

    assert!(pairing.requires_confirmation("phone-1").unwrap());
    let paired = pairing
        .pair_with_pin(
            &sess.pin,
            DeviceRecord {
                id: "phone-1".into(),
                name: "Pixel".into(),
                device_type: DeviceType::Phone,
                public_key: "pk-phone".into(),
                last_seen: 0,
                transports: vec![
                    TransportKind::Wired,
                    TransportKind::WifiLan,
                    TransportKind::Bluetooth,
                ],
                host: None,
                port: None,
                is_self: false,
                is_online: true,
                paired_at: None,
            },
        )
        .unwrap();
    assert!(pairing.is_paired("phone-1").unwrap());
    assert_eq!(
        pairing.get_pinned_public_key("phone-1").unwrap().as_deref(),
        Some("pk-phone")
    );

    // Path priority: Wired > WifiLan > HolePunched > Relay > Bluetooth
    let mut selector = PathSelector::default();
    selector.register(Arc::new(BluetoothFallbackTransport::new(None)));
    selector.register(Arc::new(UsbWiredTransport::new(
        vec!["ADB-123".into()],
        None,
    )));

    let best = selector
        .select_best(&paired, &AppSettings::default())
        .unwrap();
    assert_eq!(best.channel.kind(), TransportKind::Wired);
    assert!(best.warning.is_none());

    // Manual override to Bluetooth emits slow-transfer warning (PRD D5)
    let bt_settings = AppSettings {
        manual_transport_override: Some(TransportKind::Bluetooth),
        ..Default::default()
    };
    let forced = selector.select_best(&paired, &bt_settings).unwrap();
    assert_eq!(forced.channel.kind(), TransportKind::Bluetooth);
    assert!(forced.warning.unwrap().contains("slow"));

    pairing.forget_device("phone-1").unwrap();
    assert!(!pairing.is_paired("phone-1").unwrap());
}

#[tokio::test]
async fn real_iroh_quic_endpoints_transfer_chunks_resume_after_loss_and_verify_blake3() {
    let dir = temp_dir("mg-iroh-xfer");
    let src_file = dir.join("video_sample.mp4");
    let dest_dir = dir.join("rx");

    let mut payload = vec![0u8; 640 * 1024]; // 640 KB -> 10 chunks of 64 KB
    for (i, b) in payload.iter_mut().enumerate() {
        *b = ((i * 17) % 251) as u8;
    }
    std::fs::write(&src_file, &payload).unwrap();
    let expected_blake3 = compute_file_blake3(&src_file).unwrap();

    let receiver = Arc::new(TransferReceiver::new(&dest_dir));
    let rx_clone = Arc::clone(&receiver);

    // Bind real receiving iroh QUIC node
    let server_node = Arc::new(
        IrohQuicNode::bind(
            false,
            None,
            Arc::new(move |frame| {
                rx_clone.handle_chunk(&frame)?;
                Ok(())
            }),
            Arc::new(|_delta| Ok(())),
        )
        .await
        .unwrap(),
    );

    // Bind real sending iroh QUIC node
    let client_node = Arc::new(
        IrohQuicNode::bind(false, None, Arc::new(|_| Ok(())), Arc::new(|_| Ok(())))
            .await
            .unwrap(),
    );

    // Verify ctrl/1 ping over real iroh QUIC handshake
    let ping_reply = client_node
        .send_ctrl(
            server_node.endpoint.addr(),
            &CtrlMessage::Ping {
                from_device: "pc-1".into(),
            },
        )
        .await
        .unwrap();
    assert!(ping_reply.ok);
    assert_eq!(ping_reply.message, "pong:pc-1");

    // Set up index & pairing
    let idx = MediaIndex::open_in_memory().unwrap();
    let store = Arc::new(EncryptedMemoryStore::default());
    let pairing = Arc::new(PairingManager::new(idx.clone(), store));
    let self_dev = DeviceRecord {
        id: "pc-1".into(),
        name: "PC".into(),
        device_type: DeviceType::Pc,
        public_key: client_node.node_id_string(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: None,
        port: None,
        is_self: true,
        is_online: true,
        paired_at: None,
    };
    idx.upsert_device(&self_dev).unwrap();

    let sess = pairing
        .create_session(&self_dev, &server_node.addr_json().unwrap(), 300_000)
        .unwrap();
    let remote_peer = pairing
        .pair_with_qr(
            &sess.qr_payload,
            Some(DeviceRecord {
                id: "phone-rx".into(),
                name: "Pixel RX".into(),
                device_type: DeviceType::Phone,
                public_key: server_node.node_id_string(),
                last_seen: 1_700_000_000_000,
                transports: vec![TransportKind::WifiLan],
                host: None,
                port: None,
                is_self: false,
                is_online: true,
                paired_at: None,
            }),
        )
        .unwrap();

    let media = MediaRecord {
        id: "vid-iroh".into(),
        device_id: "pc-1".into(),
        path: src_file.to_string_lossy().to_string(),
        folder_path: dir.to_string_lossy().to_string(),
        file_name: "video_sample.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: payload.len() as u64,
        blake3: expected_blake3.clone(),
        partial_hash: expected_blake3.clone(),
        created_at: 1_773_489_600_000,
        modified_at: 1_773_489_600_000,
        duration_ms: 30_000,
        width: 1920,
        height: 1080,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: MediaTags::default(),
        alias: "Sample".into(),
        auto_alias: "Sample".into(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: 1_773_489_600_000,
    };
    idx.upsert_media(&media).unwrap();

    let quic_transport = Arc::new(IrohQuicTransport::new(
        TransportKind::WifiLan,
        Arc::clone(&client_node),
    ));
    quic_transport
        .register_peer_addr("phone-rx", server_node.endpoint.addr())
        .await;

    let engine = TransferEngine::new(idx.clone(), Some(pairing));

    // Phase 1: Simulate packet loss / drop after 4 chunks
    let faulty = SimulatedFaultyTransport::new(
        Arc::clone(&quic_transport) as Arc<dyn core_net::TransportChannel>,
        4,
    );
    let err = engine
        .send_file(
            &media,
            "pc-1",
            &remote_peer,
            &faulty,
            Some(64 * 1024),
            None,
            |_| {},
        )
        .await
        .unwrap_err();
    assert!(matches!(err, NetError::Quic(_)));

    let all_transfers = idx.list_transfers().unwrap();
    assert_eq!(all_transfers.len(), 1);
    assert_eq!(all_transfers[0].status, TransferStatus::Error);
    assert_eq!(all_transfers[0].last_chunk_index, 4);

    // Phase 2: Resume over healthy iroh QUIC channel -> completes & matches BLAKE3
    let completed = engine
        .resume_transfer(
            &all_transfers[0].id,
            &src_file,
            &remote_peer,
            quic_transport.as_ref(),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(completed.status, TransferStatus::Completed);
    assert_eq!(completed.bytes_done, payload.len() as u64);

    let received_path = receiver.completed_path(&completed.id).unwrap();
    let received_bytes = std::fs::read(&received_path).unwrap();
    assert_eq!(received_bytes, payload);

    // Tampered chunk rejected by BLAKE3 check
    let bad_rx = TransferReceiver::new(dir.join("bad_rx"));
    let mismatch = bad_rx
        .handle_chunk(&core_net::FileChunkFrame {
            transfer_id: "bad-tx".into(),
            media_id: "m-bad".into(),
            file_name: "corrupt.bin".into(),
            chunk_index: 0,
            total_chunks: 1,
            offset: 0,
            bytes_total: 5,
            expected_blake3: "0".repeat(64),
            data: b"hello".to_vec(),
        })
        .unwrap_err();
    assert!(matches!(mismatch, NetError::HashMismatch { .. }));

    client_node.shutdown().await;
    server_node.shutdown().await;
    let _ = std::fs::remove_dir_all(dir);
}
