import type { MediaRepository } from '../../core/database/repository.ts';
import type { IndexDeltaPayload } from '../../core/types/domain.ts';
import type { SecureTokenStore } from '../connectivity/pairingService.ts';

export const GOOGLE_OAUTH_SCOPES = [
  'openid',
  'email',
  'profile',
  'https://www.googleapis.com/auth/drive.appdata',
  'https://www.googleapis.com/auth/youtube.readonly',
] as const;

export interface GoogleAccountSession {
  email: string;
  accessToken: string;
  refreshToken?: string;
  expiresAt: number;
}

export interface DriveAppDataTransport {
  uploadJsonFile(fileName: string, jsonPayload: string, accessToken: string): Promise<string>;
  listAppDataFiles(accessToken: string): Promise<Array<{ id: string; name: string }>>;
  downloadJsonFile(fileId: string, accessToken: string): Promise<string>;
}

const TOKEN_KEY = 'google:oauth:session';
const CLOUD_INDEX_CACHE_KEY = 'google:drive:last_sync_meta';

/**
 * Optional Google Sign-In + Drive appDataFolder metadata/thumbnail sync (PRD E5, H1, H2, 10.4).
 * Never required for LAN operation and NEVER uploads actual media file contents.
 */
export class GoogleCloudSyncService {
  constructor(
    private readonly tokenStore: SecureTokenStore,
    private readonly repo: MediaRepository,
    private readonly driveTransport?: DriveAppDataTransport,
  ) {}

  async saveLoginSession(session: GoogleAccountSession): Promise<void> {
    await this.tokenStore.setSecret(TOKEN_KEY, JSON.stringify(session));
  }

  async getSession(): Promise<GoogleAccountSession | null> {
    const raw = await this.tokenStore.getSecret(TOKEN_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as GoogleAccountSession;
    } catch {
      return null;
    }
  }

  /**
   * Uploads metadata + small thumbnail index delta ONLY (never media file payloads) to Drive appDataFolder.
   */
  async pushIndexDeltaToDrive(delta: IndexDeltaPayload): Promise<string> {
    const session = await this.getSession();
    if (!session) {
      throw new Error('Google login is optional, but required to push cloud index deltas');
    }
    const fileName = `index-delta-${delta.device.id}.json`;
    const serialized = JSON.stringify(delta);

    let fileId = `local-sim-${delta.device.id}`;
    if (this.driveTransport) {
      fileId = await this.driveTransport.uploadJsonFile(fileName, serialized, session.accessToken);
    }

    this.repo.setSetting(
      CLOUD_INDEX_CACHE_KEY,
      JSON.stringify({ lastPushedAt: Date.now(), fileId, deviceId: delta.device.id }),
    );
    return fileId;
  }

  async pullPeerDeltasFromDrive(selfDeviceId: string): Promise<IndexDeltaPayload[]> {
    const session = await this.getSession();
    if (!session || !this.driveTransport) {
      return [];
    }
    const files = await this.driveTransport.listAppDataFiles(session.accessToken);
    const deltas: IndexDeltaPayload[] = [];
    for (const file of files) {
      if (!file.name.startsWith('index-delta-') || file.name.includes(selfDeviceId)) continue;
      const raw = await this.driveTransport.downloadJsonFile(file.id, session.accessToken);
      deltas.push(JSON.parse(raw) as IndexDeltaPayload);
    }
    return deltas;
  }

  /**
   * Logout removes OAuth tokens from OS secure storage and clears cached cloud index metadata (PRD H2).
   */
  async logoutAndWipeCloudCache(selfDeviceId: string): Promise<void> {
    await this.tokenStore.deleteSecret(TOKEN_KEY);
    this.repo.setSetting(CLOUD_INDEX_CACHE_KEY, '');

    // Remove remote-only records cached via cloud when no longer linked
    const devices = this.repo.listDevices();
    for (const dev of devices) {
      if (dev.id !== selfDeviceId && !dev.isOnline) {
        this.repo.forgetDevice(dev.id);
      }
    }
  }
}
