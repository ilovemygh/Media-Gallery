import {
  createCipheriv,
  createDecipheriv,
  createHash,
  generateKeyPairSync,
  randomBytes,
  randomInt,
} from 'node:crypto';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { DeviceRecord, DeviceType, TransportKind } from '../../core/types/domain.ts';

export interface PairingSession {
  pin: string;
  qrPayload: string;
  token: string;
  deviceId: string;
  deviceName: string;
  publicKey: string;
  expiresAt: number;
}

export interface QrPairingData {
  deviceId: string;
  deviceName: string;
  deviceType: DeviceType;
  host: string;
  port: number;
  publicKey: string;
  pinToken: string;
  expiresAt: number;
}

/**
 * Abstraction over OS Secure Storage (Electron safeStorage / Windows DPAPI / Android Keystore).
 */
export interface SecureTokenStore {
  setSecret(key: string, value: string): Promise<void>;
  getSecret(key: string): Promise<string | null>;
  deleteSecret(key: string): Promise<void>;
}

/**
 * AES-256-GCM encrypted in-memory secure credential store for tests and headless environments.
 */
export class EncryptedMemoryTokenStore implements SecureTokenStore {
  private readonly masterKey = randomBytes(32);
  private readonly entries = new Map<string, string>();

  async setSecret(key: string, value: string): Promise<void> {
    const iv = randomBytes(12);
    const cipher = createCipheriv('aes-256-gcm', this.masterKey, iv);
    const encrypted = Buffer.concat([cipher.update(value, 'utf8'), cipher.final()]);
    const tag = cipher.getAuthTag();
    this.entries.set(key, `${iv.toString('hex')}:${tag.toString('hex')}:${encrypted.toString('hex')}`);
  }

  async getSecret(key: string): Promise<string | null> {
    const packed = this.entries.get(key);
    if (!packed) return null;
    const [ivHex, tagHex, dataHex] = packed.split(':');
    const decipher = createDecipheriv(
      'aes-256-gcm',
      this.masterKey,
      Buffer.from(ivHex, 'hex'),
    );
    decipher.setAuthTag(Buffer.from(tagHex, 'hex'));
    const plain = Buffer.concat([
      decipher.update(Buffer.from(dataHex, 'hex')),
      decipher.final(),
    ]);
    return plain.toString('utf8');
  }

  async deleteSecret(key: string): Promise<void> {
    this.entries.delete(key);
  }
}

export function generateDeviceIdentityKeys(): { publicKeyPem: string; privateKeyPem: string; fingerprint: string } {
  const { publicKey, privateKey } = generateKeyPairSync('ed25519');
  const publicKeyPem = publicKey.export({ type: 'spki', format: 'pem' }).toString();
  const privateKeyPem = privateKey.export({ type: 'pkcs8', format: 'pem' }).toString();
  const fingerprint = createHash('sha256').update(publicKeyPem).digest('hex');
  return { publicKeyPem, privateKeyPem, fingerprint };
}

/**
 * Handles one-time QR code & 6-digit PIN pairing, public key pinning,
 * paired-device allow-list, and first-time sender confirmation (PRD D1, 10.2).
 */
export class PairingService {
  private activeSession: PairingSession | null = null;

  constructor(
    private readonly repo: MediaRepository,
    private readonly tokenStore: SecureTokenStore = new EncryptedMemoryTokenStore(),
  ) {}

  createPairingSession(params: {
    selfDevice: DeviceRecord;
    host: string;
    port: number;
    ttlMs?: number;
  }): PairingSession {
    const pin = String(randomInt(0, 1_000_000)).padStart(6, '0');
    const token = randomBytes(16).toString('hex');
    const expiresAt = Date.now() + (params.ttlMs ?? 5 * 60 * 1000);
    const qrData: QrPairingData = {
      deviceId: params.selfDevice.id,
      deviceName: params.selfDevice.name,
      deviceType: params.selfDevice.type,
      host: params.host,
      port: params.port,
      publicKey: params.selfDevice.publicKey,
      pinToken: `${pin}:${token}`,
      expiresAt,
    };
    this.activeSession = {
      pin,
      token,
      qrPayload: JSON.stringify(qrData),
      deviceId: params.selfDevice.id,
      deviceName: params.selfDevice.name,
      publicKey: params.selfDevice.publicKey,
      expiresAt,
    };
    return this.activeSession;
  }

  async pairUsingPin(params: {
    pin: string;
    remoteDevice: {
      id: string;
      name: string;
      type: DeviceType;
      publicKey: string;
      host?: string;
      port?: number;
      transports?: TransportKind[];
    };
  }): Promise<DeviceRecord> {
    if (!this.activeSession) {
      throw new Error('No active pairing session');
    }
    if (Date.now() > this.activeSession.expiresAt) {
      this.activeSession = null;
      throw new Error('Pairing PIN has expired');
    }
    if (params.pin.trim() !== this.activeSession.pin) {
      throw new Error('Invalid 6-digit pairing PIN');
    }

    return this.finalizePairing(params.remoteDevice, this.activeSession.token);
  }

  async pairUsingQrPayload(
    qrPayloadJson: string,
    expectedRemoteOverride?: Partial<DeviceRecord>,
  ): Promise<DeviceRecord> {
    const data = JSON.parse(qrPayloadJson) as QrPairingData;
    if (Date.now() > data.expiresAt) {
      throw new Error('Pairing QR code has expired');
    }
    return this.finalizePairing(
      {
        id: expectedRemoteOverride?.id ?? data.deviceId,
        name: expectedRemoteOverride?.name ?? data.deviceName,
        type: expectedRemoteOverride?.type ?? data.deviceType,
        publicKey: expectedRemoteOverride?.publicKey ?? data.publicKey,
        host: expectedRemoteOverride?.host ?? data.host,
        port: expectedRemoteOverride?.port ?? data.port,
        transports: expectedRemoteOverride?.transports ?? ['wifi_lan'],
      },
      data.pinToken,
    );
  }

  private async finalizePairing(
    remote: {
      id: string;
      name: string;
      type: DeviceType;
      publicKey: string;
      host?: string;
      port?: number;
      transports?: TransportKind[];
    },
    sharedSecret: string,
  ): Promise<DeviceRecord> {
    const now = Date.now();
    const record: DeviceRecord = {
      id: remote.id,
      name: remote.name,
      type: remote.type,
      publicKey: remote.publicKey,
      lastSeen: now,
      transports: remote.transports ?? ['wifi_lan'],
      host: remote.host,
      port: remote.port,
      isSelf: false,
      isOnline: true,
      pairedAt: now,
    };
    this.repo.upsertDevice(record);
    await this.tokenStore.setSecret(`peer:${remote.id}:auth`, sharedSecret);
    await this.tokenStore.setSecret(`peer:${remote.id}:pubkey`, remote.publicKey);
    this.activeSession = null;
    return record;
  }

  isPaired(deviceId: string): boolean {
    const dev = this.repo.getDevice(deviceId);
    return Boolean(dev && dev.pairedAt != null && dev.publicKey.length > 0);
  }

  requiresTransferConfirmation(senderDeviceId: string): boolean {
    return !this.isPaired(senderDeviceId);
  }

  async getPinnedPublicKey(deviceId: string): Promise<string | null> {
    return (await this.tokenStore.getSecret(`peer:${deviceId}:pubkey`)) ?? this.repo.getDevice(deviceId)?.publicKey ?? null;
  }

  async forgetDevice(deviceId: string): Promise<void> {
    this.repo.forgetDevice(deviceId);
    await this.tokenStore.deleteSecret(`peer:${deviceId}:auth`);
    await this.tokenStore.deleteSecret(`peer:${deviceId}:pubkey`);
  }
}
