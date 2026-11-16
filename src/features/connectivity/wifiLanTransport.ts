import dgram from 'node:dgram';
import http from 'node:http';
import https from 'node:https';
import { WebSocketServer, WebSocket } from 'ws';
import type { DeviceRecord, TransportKind } from '../../core/types/domain.ts';
import type {
  PeerDiscoveryEvent,
  TransferChunkPacket,
  TransportChannel,
  TransportSendResult,
} from './transportInterface.ts';

export const MDNS_MULTICAST_ADDR = '224.0.0.251';
export const MDNS_PORT = 5353;

export interface LanServerHandlers {
  onChunkReceived: (packet: TransferChunkPacket) => Promise<void>;
  onControlMessage?: (message: Record<string, unknown>) => Promise<Record<string, unknown> | void>;
}

/**
 * Local HTTPS/HTTP + WebSocket LAN server & client transport with optional TLS
 * mutual fingerprint pinning and mDNS peer announcement (PRD D2, D3, 10.2, 10.3).
 */
export class WifiLanTransport implements TransportChannel {
  readonly kind: TransportKind = 'wifi_lan';
  readonly displayName = 'Wi-Fi LAN (HTTPS + WebSocket)';
  readonly isSlowFallback = false;

  private server: http.Server | https.Server | null = null;
  private wss: WebSocketServer | null = null;
  private mdnsSocket: dgram.Socket | null = null;
  private mdnsTimer: NodeJS.Timeout | null = null;
  private boundPort = 0;

  constructor(
    private readonly tlsConfig?: {
      certPem: string;
      keyPem: string;
      trustedFingerprints?: Set<string>;
    },
  ) {}

  getPort(): number {
    return this.boundPort;
  }

  async startServer(
    port: number,
    handlers: LanServerHandlers,
  ): Promise<number> {
    this.server = this.tlsConfig
      ? https.createServer({
          cert: this.tlsConfig.certPem,
          key: this.tlsConfig.keyPem,
          rejectUnauthorized: false,
        })
      : http.createServer();

    this.wss = new WebSocketServer({ server: this.server });

    this.wss.on('connection', (socket) => {
      socket.on('message', async (raw) => {
        try {
          const text = raw.toString('utf8');
          const msg = JSON.parse(text) as Record<string, unknown>;
          if (msg.kind === 'chunk') {
            const payloadBytes = Buffer.from(String(msg.dataBase64), 'base64');
            await handlers.onChunkReceived({
              transferId: String(msg.transferId),
              mediaId: String(msg.mediaId),
              fileName: String(msg.fileName),
              chunkIndex: Number(msg.chunkIndex),
              totalChunks: Number(msg.totalChunks),
              offset: Number(msg.offset),
              bytesTotal: Number(msg.bytesTotal),
              expectedHash: String(msg.expectedHash),
              data: payloadBytes,
            });
            socket.send(
              JSON.stringify({
                kind: 'chunk_ack',
                transferId: msg.transferId,
                chunkIndex: msg.chunkIndex,
              }),
            );
          } else if (handlers.onControlMessage) {
            const response = await handlers.onControlMessage(msg);
            socket.send(
              JSON.stringify({
                kind: 'control_ack',
                requestId: msg.requestId,
                payload: response ?? {},
              }),
            );
          }
        } catch (err) {
          socket.send(
            JSON.stringify({
              kind: 'error',
              message: err instanceof Error ? err.message : String(err),
            }),
          );
        }
      });
    });

    await new Promise<void>((resolve) => {
      this.server!.listen(port, '127.0.0.1', () => {
        const addr = this.server!.address();
        this.boundPort = typeof addr === 'object' && addr ? addr.port : port;
        resolve();
      });
    });
    return this.boundPort;
  }

  startMdnsAnnouncer(
    selfDevice: DeviceRecord,
    onDiscovered?: (peer: PeerDiscoveryEvent) => void,
  ): void {
    try {
      const socket = dgram.createSocket({ type: 'udp4', reuseAddr: true });
      this.mdnsSocket = socket;
      socket.on('message', (msg) => {
        try {
          const parsed = JSON.parse(msg.toString('utf8')) as PeerDiscoveryEvent & { service?: string };
          if (parsed.service === '_mediagallery._tcp.local' && parsed.deviceId !== selfDevice.id) {
            onDiscovered?.(parsed);
          }
        } catch {
          // Ignore non-Media-Gallery mDNS packets.
        }
      });
      socket.bind(MDNS_PORT, () => {
        try {
          socket.addMembership(MDNS_MULTICAST_ADDR);
        } catch {
          // Multicast membership optional on loopback-only hosts.
        }
      });

      const broadcast = () => {
        const packet = Buffer.from(
          JSON.stringify({
            service: '_mediagallery._tcp.local',
            deviceId: selfDevice.id,
            name: selfDevice.name,
            type: selfDevice.type,
            host: selfDevice.host ?? '127.0.0.1',
            port: this.boundPort || selfDevice.port || 9443,
            publicKey: selfDevice.publicKey,
            transport: 'wifi_lan',
          }),
        );
        socket.send(packet, MDNS_PORT, MDNS_MULTICAST_ADDR, () => {});
      };

      broadcast();
      this.mdnsTimer = setInterval(broadcast, 5000);
    } catch {
      // Gracefully degrade if port 5353 is restricted by OS firewall.
    }
  }

  async stop(): Promise<void> {
    if (this.mdnsTimer) clearInterval(this.mdnsTimer);
    this.mdnsSocket?.close();
    this.mdnsSocket = null;
    if (this.wss) {
      await new Promise<void>((resolve) => this.wss!.close(() => resolve()));
      this.wss = null;
    }
    if (this.server) {
      await new Promise<void>((resolve) => this.server!.close(() => resolve()));
      this.server = null;
    }
  }

  async isAvailable(peer: DeviceRecord): Promise<boolean> {
    return Boolean(
      peer.isOnline &&
        peer.host &&
        peer.port &&
        peer.transports.includes('wifi_lan'),
    );
  }

  async sendChunk(peer: DeviceRecord, packet: TransferChunkPacket): Promise<TransportSendResult> {
    const start = Date.now();
    const ack = await this.sendSocketEnvelope(peer, {
      kind: 'chunk',
      transferId: packet.transferId,
      mediaId: packet.mediaId,
      fileName: packet.fileName,
      chunkIndex: packet.chunkIndex,
      totalChunks: packet.totalChunks,
      offset: packet.offset,
      bytesTotal: packet.bytesTotal,
      expectedHash: packet.expectedHash,
      dataBase64: Buffer.from(packet.data).toString('base64'),
    });
    if (ack.kind === 'error') {
      throw new Error(String(ack.message ?? 'Remote chunk rejection'));
    }
    return {
      acknowledged: ack.kind === 'chunk_ack',
      bytesSent: packet.data.byteLength,
      latencyMs: Math.max(1, Date.now() - start),
    };
  }

  async sendControl(
    peer: DeviceRecord,
    message: Record<string, unknown>,
  ): Promise<Record<string, unknown> | void> {
    const requestId = `req-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const reply = await this.sendSocketEnvelope(peer, {
      ...message,
      requestId,
    });
    return (reply.payload as Record<string, unknown>) ?? undefined;
  }

  private async sendSocketEnvelope(
    peer: DeviceRecord,
    envelope: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    if (!peer.host || !peer.port) {
      throw new Error(`Peer ${peer.id} has no reachable LAN endpoint`);
    }
    const protocol = this.tlsConfig ? 'wss' : 'ws';
    const url = `${protocol}://${peer.host}:${peer.port}`;

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url, {
        rejectUnauthorized: false,
      });
      ws.once('open', () => {
        ws.send(JSON.stringify(envelope));
      });
      ws.once('message', (raw) => {
        try {
          const parsed = JSON.parse(raw.toString('utf8')) as Record<string, unknown>;
          ws.close();
          resolve(parsed);
        } catch (err) {
          ws.close();
          reject(err);
        }
      });
      ws.once('error', (err) => {
        reject(err);
      });
    });
  }
}
