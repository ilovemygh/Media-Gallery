import type { DeviceRecord, TransportKind } from '../../core/types/domain.ts';
import type {
  TransferChunkPacket,
  TransportChannel,
  TransportSendResult,
} from './transportInterface.ts';

export interface BluetoothSocketBridge {
  isAdapterReady(peerId: string): Promise<boolean>;
  writePacket(peerId: string, payload: Uint8Array): Promise<void>;
}

/**
 * Bluetooth Classic / BLE fallback transport (PRD D5, 10.3).
 * Recommended for pairing and small/medium files; surfaces a slow-transfer warning to UI.
 */
export class BluetoothTransport implements TransportChannel {
  readonly kind: TransportKind = 'bluetooth';
  readonly displayName = 'Bluetooth (Fallback)';
  readonly isSlowFallback = true;
  readonly slowWarning =
    'Bluetooth transfer is slow (~1–2 MB/s) and best suited for small/medium files. Connect USB or Wi-Fi LAN for faster speed.';

  constructor(private readonly bridge?: BluetoothSocketBridge) {}

  async isAvailable(peer: DeviceRecord): Promise<boolean> {
    if (!peer.isOnline || !peer.transports.includes('bluetooth')) {
      return false;
    }
    if (this.bridge) {
      return this.bridge.isAdapterReady(peer.id);
    }
    return true;
  }

  async sendChunk(peer: DeviceRecord, packet: TransferChunkPacket): Promise<TransportSendResult> {
    const start = Date.now();
    if (this.bridge) {
      await this.bridge.writePacket(peer.id, packet.data);
    }
    return {
      acknowledged: true,
      bytesSent: packet.data.byteLength,
      latencyMs: Math.max(5, Date.now() - start),
    };
  }

  async sendControl(
    peer: DeviceRecord,
    message: Record<string, unknown>,
  ): Promise<Record<string, unknown> | void> {
    if (this.bridge) {
      await this.bridge.writePacket(peer.id, Buffer.from(JSON.stringify(message), 'utf8'));
    }
  }
}
