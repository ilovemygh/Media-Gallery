import type { MediaRecord, MediaTags, MediaType } from '../src/core/types/domain.ts';
import { generateAutoAlias } from '../src/features/alias/aliasEngine.ts';

export interface AndroidMediaStoreRow {
  uri: string;
  displayName: string;
  relativePath: string;
  mimeType: string;
  size: number;
  dateAddedSec: number;
  dateModifiedSec: number;
  durationMs?: number;
  width?: number;
  height?: number;
  latitude?: number | null;
  longitude?: number | null;
  artist?: string;
  album?: string;
  title?: string;
}

export interface AndroidShareIntentPayload {
  uris: string[];
  mimeType: string;
  action: 'ACTION_SEND' | 'ACTION_SEND_MULTIPLE';
}

export interface NativeAndroidBridge {
  scanMediaStore(sinceTimestampSec: number): Promise<AndroidMediaStoreRow[]>;
  extractVideoFrame(uri: string, frameTimeMs: number, outputFilePath: string): Promise<string>;
  startForegroundNotification(title: string, body: string, kind: 'transfer' | 'download' | 'playback'): Promise<void>;
  stopForegroundNotification(): Promise<void>;
  registerShareListener(handler: (payload: AndroidShareIntentPayload) => void): () => void;
  downloadYoutubeWithNativeLib(url: string, format: 'mp4' | 'mp3', quality: string, destDir: string): Promise<string>;
}

function inferMediaType(mimeType: string): MediaType {
  if (mimeType.startsWith('video/')) return 'video';
  if (mimeType.startsWith('audio/')) return 'audio';
  return 'image';
}

/**
 * Normalizes Android MediaStore rows into Media Gallery index records (PRD Section 4 & 8).
 */
export function mapMediaStoreRowsToMediaRecords(
  rows: AndroidMediaStoreRow[],
  deviceId: string,
  deviceName: string,
): MediaRecord[] {
  return rows.map((row, idx) => {
    const type = inferMediaType(row.mimeType);
    const createdAt = (row.dateAddedSec || row.dateModifiedSec) * 1000;
    const modifiedAt = row.dateModifiedSec * 1000;
    const tags: MediaTags = {
      artist: row.artist,
      album: row.album,
      title: row.title,
    };
    const { autoAlias, resolvedPlace } = generateAutoAlias({
      fileName: row.displayName,
      type,
      createdAt,
      modifiedAt,
      gpsLat: row.latitude ?? null,
      gpsLon: row.longitude ?? null,
      tags,
      deviceName,
    });
    const folderPath = `/storage/emulated/0/${row.relativePath.replace(/\/+$/, '')}`;
    return {
      id: `and-${deviceId}-${idx}-${row.displayName}`,
      deviceId,
      path: row.uri,
      folderPath,
      fileName: row.displayName,
      type,
      mime: row.mimeType,
      size: row.size,
      hash: '',
      partialHash: `${row.size}:${row.dateModifiedSec}`,
      createdAt,
      modifiedAt,
      durationMs: row.durationMs ?? 0,
      width: row.width ?? 0,
      height: row.height ?? 0,
      gpsLat: row.latitude ?? null,
      gpsLon: row.longitude ?? null,
      place: resolvedPlace,
      tags,
      alias: autoAlias,
      autoAlias,
      coverId: null,
      syncState: 'Local',
      isTrashed: false,
      updatedAt: Date.now(),
    };
  });
}
