import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type { DeviceRecord, TransportKind } from '../../core/types/domain.ts';
import type {
  TransferChunkPacket,
  TransportChannel,
  TransportSendResult,
} from './transportInterface.ts';
import { WifiLanTransport } from './wifiLanTransport.ts';

const execFileAsync = promisify(execFile);

export interface AdbClient {
  listConnectedSerials(): Promise<string[]>;
  setupPortForward(serial: string, localPort: number, remotePort: number): Promise<void>;
}

const defaultAdbClient: AdbClient = {
  async listConnectedSerials() {
    try {
      const { stdout } = await execFileAsync('adb', ['devices']);
      return stdout
        .split(/\r?\n/)
        .slice(1)
        .map((line) => line.trim())
        .filter((line) => line.endsWith('\tdevice'))
        .map((line) => line.split('\t')[0]);
    } catch {
      return [];
    }
  },
  async setupPortForward(serial, localPort, remotePort) {
    await execFileAsync('adb', ['-s', serial, 'forward', `tcp:${localPort}`, `tcp:${remotePort}`]);
  },
};

/**
 * Wired USB Transport (PRD D4, 10.3):
 * Uses ADB port-forwarding or USB tethering to expose the same secure chunked
 * protocol over loopback/USB RNDIS, with MTP as an explicit fallback indicator.
 */
export class UsbWiredTransport implements TransportChannel {
  readonly kind: TransportKind = 'wired';
  readonly displayName = 'Wired USB (ADB / Tethering)';
  readonly isSlowFallback = false;

  private readonly tunnelTransport = new WifiLanTransport();

  constructor(
    private readonly adb: AdbClient = defaultAdbClient,
    private readonly localForwardPort = 9443,
  ) {}

  async isAvailable(peer: DeviceRecord): Promise<boolean> {
    if (!peer.isOnline || !peer.transports.includes('wired')) {
      return false;
    }
    if (peer.host === '127.0.0.1' && peer.port) {
      return true;
    }
    const serials = await this.adb.listConnectedSerials();
    return serials.length > 0;
  }

  async ensureAdbTunnel(remotePort = 9443): Promise<number> {
    const serials = await this.adb.listConnectedSerials();
    if (serials.length === 0) {
      throw new Error('No Android device detected over ADB USB (MTP fallback available for manual copy)');
    }
    await this.adb.setupPortForward(serials[0], this.localForwardPort, remotePort);
    return this.localForwardPort;
  }

  async sendChunk(peer: DeviceRecord, packet: TransferChunkPacket): Promise<TransportSendResult> {
    const tunneledPeer: DeviceRecord = {
      ...peer,
      host: peer.host ?? '127.0.0.1',
      port: peer.port ?? this.localForwardPort,
    };
    return this.tunnelTransport.sendChunk(tunneledPeer, packet);
  }

  async sendControl(
    peer: DeviceRecord,
    message: Record<string, unknown>,
  ): Promise<Record<string, unknown> | void> {
    const tunneledPeer: DeviceRecord = {
      ...peer,
      host: peer.host ?? '127.0.0.1',
      port: peer.port ?? this.localForwardPort,
    };
    return this.tunnelTransport.sendControl(tunneledPeer, message);
  }
}
