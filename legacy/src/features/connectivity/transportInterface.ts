import type { DeviceRecord, TransportKind } from '../../core/types/domain.ts';

export interface TransferChunkPacket {
  transferId: string;
  mediaId: string;
  fileName: string;
  chunkIndex: number;
  totalChunks: number;
  offset: number;
  bytesTotal: number;
  expectedHash: string;
  data: Uint8Array;
}

export interface TransportSendResult {
  acknowledged: boolean;
  bytesSent: number;
  latencyMs: number;
}

export interface PeerDiscoveryEvent {
  deviceId: string;
  name: string;
  type: DeviceRecord['type'];
  host: string;
  port: number;
  publicKey: string;
  transport: TransportKind;
}

/**
 * Modular transport contract implemented independently by Wi-Fi LAN,
 * USB (ADB/tethering), Wi-Fi Direct, and Bluetooth channels (PRD D4–D6).
 */
export interface TransportChannel {
  readonly kind: TransportKind;
  readonly displayName: string;
  readonly isSlowFallback: boolean;
  readonly slowWarning?: string;

  isAvailable(peer: DeviceRecord): Promise<boolean>;
  sendChunk(peer: DeviceRecord, packet: TransferChunkPacket): Promise<TransportSendResult>;
  sendControl(peer: DeviceRecord, message: Record<string, unknown>): Promise<Record<string, unknown> | void>;
}
