import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import type { DeviceRecord, MediaRecord } from '../src/core/types/domain.ts';
import { computeFileSha256 } from '../src/core/utils/hash.ts';
import { BluetoothTransport } from '../src/features/connectivity/bluetoothTransport.ts';
import {
  EncryptedMemoryTokenStore,
  generateDeviceIdentityKeys,
  PairingService,
} from '../src/features/connectivity/pairingService.ts';
import {
  TransferEngine,
  TransferReceiver,
} from '../src/features/connectivity/transferEngine.ts';
import { TransportManager } from '../src/features/connectivity/transportManager.ts';
import { UsbWiredTransport } from '../src/features/connectivity/usbTransport.ts';
import { WifiLanTransport } from '../src/features/connectivity/wifiLanTransport.ts';
import { GoogleCloudSyncService } from '../src/features/sync/googleCloudSync.ts';
import { IndexSyncService } from '../src/features/sync/indexSyncService.ts';

describe('Pairing, Transport Priority, Resumable Transfers & Serverless Sync (M3, M4, M6)', () => {
  const tempDirs: string[] = [];

  afterEach(async () => {
    for (const dir of tempDirs.splice(0)) {
      await rm(dir, { recursive: true, force: true });
    }
  });

  function setupRepo() {
    const repo = new MediaRepository(new NodeSqliteAdapter(':memory:'));
    const keys = generateDeviceIdentityKeys();
    const self: DeviceRecord = {
      id: 'pc-1',
      name: 'PC',
      type: 'pc',
      publicKey: keys.fingerprint,
      lastSeen: Date.now(),
      transports: ['wired', 'wifi_lan', 'bluetooth'],
      host: '127.0.0.1',
      port: 9443,
      isSelf: true,
      isOnline: true,
    };
    repo.upsertDevice(self);
    return { repo, self };
  }

  it('pairs phone and PC via QR/PIN, stores pinned key securely, and blocks unpaired sends (D1, 10.2)', async () => {
    const { repo, self } = setupRepo();
    const tokenStore = new EncryptedMemoryTokenStore();
    const pairing = new PairingService(repo, tokenStore);

    const session = pairing.createPairingSession({
      selfDevice: self,
      host: '127.0.0.1',
      port: 9443,
    });
    expect(session.pin).toMatch(/^\d{6}$/);

    // Reject wrong PIN
    await expect(
      pairing.pairUsingPin({
        pin: '000000' === session.pin ? '111111' : '000000',
        remoteDevice: { id: 'phone-1', name: 'Pixel', type: 'phone', publicKey: 'pk-phone' },
      }),
    ).rejects.toThrow(/Invalid 6-digit/);

    // Unpaired device requires user confirmation
    expect(pairing.requiresTransferConfirmation('phone-1')).toBe(true);

    const paired = await pairing.pairUsingPin({
      pin: session.pin,
      remoteDevice: { id: 'phone-1', name: 'Pixel', type: 'phone', publicKey: 'pk-phone' },
    });
    expect(paired.pairedAt).toBeTypeOf('number');
    expect(pairing.isPaired('phone-1')).toBe(true);
    expect(await pairing.getPinnedPublicKey('phone-1')).toBe('pk-phone');

    // Forget removes peer and stored token
    await pairing.forgetDevice('phone-1');
    expect(pairing.isPaired('phone-1')).toBe(false);
    expect(await tokenStore.getSecret('peer:phone-1:auth')).toBeNull();

    repo.close();
  });

  it('selects fastest transport automatically (wired > wifi_lan > bluetooth) with warning for Bluetooth (D4–D6)', async () => {
    const manager = new TransportManager();
    const usb = new UsbWiredTransport({
      listConnectedSerials: async () => ['PIXEL-ADB-01'],
      setupPortForward: async () => {},
    });
    const wifi = new WifiLanTransport();
    const bt = new BluetoothTransport();

    manager.registerChannel(bt);
    manager.registerChannel(wifi);
    manager.registerChannel(usb);

    const peer: DeviceRecord = {
      id: 'phone-1',
      name: 'Pixel 9',
      type: 'phone',
      publicKey: 'pk',
      lastSeen: Date.now(),
      transports: ['wired', 'wifi_lan', 'bluetooth'],
      host: '192.168.1.50',
      port: 9443,
      isOnline: true,
    };

    const best = await manager.selectBestTransport(peer);
    expect(best.channel.kind).toBe('wired');
    expect(best.warning).toBeNull();

    // When wired is unavailable, falls back to wifi_lan
    const wifiOnlyPeer: DeviceRecord = { ...peer, transports: ['wifi_lan', 'bluetooth'] };
    const second = await manager.selectBestTransport(wifiOnlyPeer);
    expect(second.channel.kind).toBe('wifi_lan');

    // Manual override to Bluetooth triggers slow-speed warning
    const forcedBt = await manager.selectBestTransport(peer, {
      transportPriority: ['wired', 'wifi_lan', 'bluetooth'],
      manualTransportOverride: 'bluetooth',
    });
    expect(forcedBt.channel.kind).toBe('bluetooth');
    expect(forcedBt.warning).toContain('slow');
  });

  it('transfers file in chunks over real WebSocket LAN, resumes after interruption, and verifies SHA-256 (D7)', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'mg-xfer-'));
    tempDirs.push(dir);

    const sourceFile = path.join(dir, 'big_clip.mp4');
    const destDir = path.join(dir, 'received');
    const sourcePayload = Buffer.alloc(600 * 1024); // 600 KB -> 10 chunks of 64KB
    for (let i = 0; i < sourcePayload.length; i += 1) {
      sourcePayload[i] = (i * 31) % 256;
    }
    await writeFile(sourceFile, sourcePayload);
    const expectedHash = await computeFileSha256(sourceFile);

    const { repo, self } = setupRepo();
    const pairing = new PairingService(repo);
    const qrSession = pairing.createPairingSession({ selfDevice: self, host: '127.0.0.1', port: 0 });

    const receiver = new TransferReceiver(destDir);
    const serverTransport = new WifiLanTransport();
    const port = await serverTransport.startServer(0, {
      onChunkReceived: async (packet) => {
        await receiver.handleIncomingChunk(packet);
      },
    });

    const remotePhone = await pairing.pairUsingQrPayload(qrSession.qrPayload, {
      id: 'phone-rx',
      name: 'Pixel RX',
      type: 'phone',
      host: '127.0.0.1',
      port,
      transports: ['wifi_lan'],
    });

    const media: MediaRecord = {
      id: 'vid-xfer',
      deviceId: 'pc-1',
      path: sourceFile,
      folderPath: dir.replace(/\\/g, '/'),
      fileName: 'big_clip.mp4',
      type: 'video',
      mime: 'video/mp4',
      size: sourcePayload.length,
      hash: expectedHash,
      createdAt: Date.now(),
      modifiedAt: Date.now(),
      durationMs: 30_000,
      width: 1920,
      height: 1080,
      gpsLat: null,
      gpsLon: null,
      place: null,
      tags: {},
      alias: 'Big Clip',
      autoAlias: 'Big Clip',
      coverId: null,
      syncState: 'Local',
    };
    repo.upsertMedia(media);

    const clientTransport = new WifiLanTransport();
    const engine = new TransferEngine(repo, pairing);

    // Phase 1: send only 3 chunks then simulate interruption
    const interrupted = await engine.sendFile({
      media,
      fromDeviceId: 'pc-1',
      toDevice: remotePhone,
      transport: clientTransport,
      chunkSize: 64 * 1024,
      maxChunksPerRun: 3,
    });
    expect(interrupted.status).toBe('paused');
    expect(interrupted.lastChunkIndex).toBe(3);
    expect(interrupted.bytesDone).toBe(3 * 64 * 1024);

    // Phase 2: resume from chunk 3 to completion
    const completed = await engine.resumeTransfer({
      transferId: interrupted.id,
      sourceFilePath: sourceFile,
      toDevice: remotePhone,
      transport: clientTransport,
    });
    expect(completed.status).toBe('completed');
    expect(completed.bytesDone).toBe(sourcePayload.length);

    const receivedFile = receiver.getCompletedFilePath(interrupted.id)!;
    expect(receivedFile).toBeDefined();
    const receivedBytes = await readFile(receivedFile);
    expect(Buffer.compare(receivedBytes, sourcePayload)).toBe(0);

    // Verify receiver rejects tampered chunk SHA-256 mismatch
    const badReceiver = new TransferReceiver(path.join(dir, 'bad-rx'));
    await expect(
      badReceiver.handleIncomingChunk({
        transferId: 'bad-1',
        mediaId: 'm-bad',
        fileName: 'bad.bin',
        chunkIndex: 0,
        totalChunks: 1,
        offset: 0,
        bytesTotal: 5,
        expectedHash: '0'.repeat(64),
        data: Buffer.from('hello'),
      }),
    ).rejects.toThrow(/SHA-256 verification failed/);

    await serverTransport.stop();
    repo.close();
  });

  it('syncs metadata & thumbnails without file contents and disables Download when owner offline (E1–E6, H2)', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'mg-sync-'));
    tempDirs.push(dir);
    const thumbFile = path.join(dir, 'phone-thumb.svg');
    await writeFile(thumbFile, '<svg xmlns="http://www.w3.org/2000/svg"></svg>');

    // Phone repo exports delta
    const phoneRepo = new MediaRepository(new NodeSqliteAdapter(':memory:'));
    const phoneDevice: DeviceRecord = {
      id: 'phone-src',
      name: 'Pixel 9',
      type: 'phone',
      publicKey: 'pk-phone',
      lastSeen: Date.now(),
      transports: ['wifi_lan'],
      host: '192.168.1.90',
      port: 9443,
      isSelf: true,
      isOnline: true,
    };
    phoneRepo.upsertDevice(phoneDevice);
    phoneRepo.upsertMedia({
      id: 'pm-1',
      deviceId: 'phone-src',
      path: '/storage/emulated/0/DCIM/Camera/VID_01.mp4',
      folderPath: '/storage/emulated/0/DCIM/Camera',
      fileName: 'VID_01.mp4',
      type: 'video',
      mime: 'video/mp4',
      size: 50_000_000,
      hash: 'h-pm1',
      createdAt: Date.now(),
      modifiedAt: Date.now(),
      durationMs: 60_000,
      width: 1920,
      height: 1080,
      gpsLat: null,
      gpsLon: null,
      place: null,
      tags: {},
      alias: 'Phone Clip',
      autoAlias: 'Phone Clip',
      coverId: 'cov-pm1',
      syncState: 'Local',
      updatedAt: Date.now(),
    });
    phoneRepo.upsertCover({
      id: 'cov-pm1',
      mediaId: 'pm-1',
      source: 'generated',
      filePath: thumbFile,
      cloudId: null,
      aspect: '16:9',
      crop: null,
      frameTimeMs: 12000,
      updatedAt: Date.now(),
    });

    const phoneSync = new IndexSyncService(phoneRepo);
    const delta = await phoneSync.exportLocalDelta('phone-src', 0, true);
    expect(delta.thumbnailBlobs['pm-1']).toContain('data:image/svg+xml;base64,');

    // PC repo imports delta on LAN (no Google login required, E5)
    const { repo: pcRepo } = setupRepo();
    const pcSync = new IndexSyncService(pcRepo);
    const { importedCount } = await pcSync.importRemoteDelta(
      delta,
      'pc-1',
      path.join(dir, 'pc-remote-cache'),
    );
    expect(importedCount).toBe(1);

    const remoteItem = pcRepo.getMediaById('pm-1')!;
    expect(remoteItem.syncState).toBe('Remote');
    expect(pcSync.getLanStreamPreviewUrl(remoteItem)).toContain('192.168.1.90:9443');

    // Owner goes offline -> browsable via cached thumbnail, Download disabled cleanly (E6)
    pcRepo.updateDeviceOnlineState('phone-src', false);
    const offlineItem = pcRepo.getMediaById('pm-1')!;
    expect(() => pcSync.assertDownloadAllowed(offlineItem)).toThrow(/offline/i);

    // Optional Google Cloud Drive sync + logout token/cache wipe (H2)
    const tokenStore = new EncryptedMemoryTokenStore();
    const cloud = new GoogleCloudSyncService(tokenStore, pcRepo);
    await cloud.saveLoginSession({
      email: 'user@example.com',
      accessToken: 'tok-123',
      expiresAt: Date.now() + 3600_000,
    });
    await cloud.pushIndexDeltaToDrive(delta);
    expect(await cloud.getSession()).not.toBeNull();

    await cloud.logoutAndWipeCloudCache('pc-1');
    expect(await cloud.getSession()).toBeNull();
    expect(pcRepo.getMediaById('pm-1')).toBeNull();

    phoneRepo.close();
    pcRepo.close();
  });
});
