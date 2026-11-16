import type { AppSettings, DeviceRecord, TransportKind } from '../../core/types/domain.ts';
import type { TransportChannel } from './transportInterface.ts';

export const DEFAULT_TRANSPORT_PRIORITY: TransportKind[] = [
  'wired',
  'wifi_lan',
  'wifi_direct',
  'bluetooth',
];

export interface SelectedTransportResult {
  channel: TransportChannel;
  warning: string | null;
}

/**
 * Selects the fastest available transport automatically (wired > wifi_lan > wifi_direct > bluetooth)
 * while respecting optional manual overrides in settings (PRD D6).
 */
export class TransportManager {
  private readonly channels = new Map<TransportKind, TransportChannel>();

  registerChannel(channel: TransportChannel): void {
    this.channels.set(channel.kind, channel);
  }

  listRegisteredKinds(): TransportKind[] {
    return Array.from(this.channels.keys());
  }

  async selectBestTransport(
    peer: DeviceRecord,
    settings?: Pick<AppSettings, 'transportPriority' | 'manualTransportOverride'>,
  ): Promise<SelectedTransportResult> {
    const manual = settings?.manualTransportOverride;
    if (manual && manual !== 'auto') {
      const explicit = this.channels.get(manual);
      if (explicit && (await explicit.isAvailable(peer))) {
        return {
          channel: explicit,
          warning: explicit.isSlowFallback ? (explicit.slowWarning ?? null) : null,
        };
      }
      throw new Error(`Manually selected transport "${manual}" is not available for ${peer.name}`);
    }

    const priority =
      settings?.transportPriority && settings.transportPriority.length > 0
        ? settings.transportPriority
        : DEFAULT_TRANSPORT_PRIORITY;

    for (const kind of priority) {
      const channel = this.channels.get(kind);
      if (!channel) continue;
      if (await channel.isAvailable(peer)) {
        return {
          channel,
          warning: channel.isSlowFallback ? (channel.slowWarning ?? null) : null,
        };
      }
    }

    throw new Error(`No reachable transport available for device "${peer.name}"`);
  }
}
