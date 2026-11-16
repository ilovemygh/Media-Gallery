import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { access, mkdir, readdir, stat, unlink, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { AspectPreset, CoverRecord, MediaRecord } from '../../core/types/domain.ts';

const execFileAsync = promisify(execFile);

export interface ThumbnailOptions {
  cacheDir: string;
  ffmpegPath?: string;
  useFfmpeg?: boolean;
  maxCacheBytes?: number;
  rng?: () => number;
}

/**
 * Selects a random frame timestamp within the first third [0, durationMs / 3]
 * of a video (PRD I1).
 */
export function selectRandomFirstThirdFrameMs(
  durationMs: number,
  rng: () => number = Math.random,
): number {
  if (!Number.isFinite(durationMs) || durationMs <= 0) {
    return 0;
  }
  const firstThirdEndMs = Math.floor(durationMs / 3);
  if (firstThirdEndMs <= 0) return 0;
  const clampedSample = Math.min(0.999999999, Math.max(0, rng()));
  return Math.floor(clampedSample * (firstThirdEndMs + 1));
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function buildFallbackCoverSvg(media: MediaRecord, frameTimeMs: number): string {
  const palette =
    media.type === 'video'
      ? ['#1d4ed8', '#0f172a']
      : media.type === 'audio'
        ? ['#7c3aed', '#1e1b4b']
        : ['#0f766e', '#042f2e'];
  const title = escapeXml((media.alias || media.autoAlias || media.fileName).slice(0, 32));
  const subtitle =
    media.type === 'video'
      ? `Frame @ ${(frameTimeMs / 1000).toFixed(2)}s`
      : media.type === 'audio'
        ? escapeXml(String(media.tags?.artist ?? 'Audio Track'))
        : `${media.width || 800}x${media.height || 600}`;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="270" viewBox="0 0 480 270">
  <defs>
    <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="${palette[0]}"/>
      <stop offset="100%" stop-color="${palette[1]}"/>
    </linearGradient>
  </defs>
  <rect width="480" height="270" fill="url(#g)"/>
  <circle cx="420" cy="52" r="24" fill="rgba(255,255,255,0.12)"/>
  <text x="24" y="198" fill="#f8fafc" font-family="Segoe UI, sans-serif" font-size="22" font-weight="700">${title}</text>
  <text x="24" y="230" fill="#cbd5e1" font-family="Segoe UI, sans-serif" font-size="14">${subtitle}</text>
</svg>`;
}

/**
 * Generates and caches thumbnails while keeping video frame selections fixed once created.
 */
export class ThumbnailService {
  constructor(private readonly repo: MediaRepository) {}

  async ensureThumbnail(media: MediaRecord, options: ThumbnailOptions): Promise<CoverRecord> {
    const existing = this.repo.getCoverByMediaId(media.id);
    if (existing) {
      return existing;
    }

    await mkdir(options.cacheDir, { recursive: true });
    const frameTimeMs =
      media.type === 'video'
        ? selectRandomFirstThirdFrameMs(media.durationMs, options.rng)
        : 0;

    const aspect: AspectPreset = media.type === 'audio' ? '1:1' : '16:9';
    const thumbPath = await this.renderThumbnailFile(media, frameTimeMs, options);

    const cover: CoverRecord = {
      id: randomUUID(),
      mediaId: media.id,
      source: media.type === 'audio' && media.tags?.embeddedCoverBackup ? 'embedded' : 'generated',
      filePath: thumbPath,
      cloudId: null,
      aspect,
      crop: null,
      frameTimeMs,
      updatedAt: Date.now(),
    };

    this.repo.upsertCover(cover);

    if (options.maxCacheBytes && options.maxCacheBytes > 0) {
      await this.enforceLruCacheLimit(options.cacheDir, options.maxCacheBytes, new Set([thumbPath]));
    }

    return cover;
  }

  async renderThumbnailFile(
    media: MediaRecord,
    frameTimeMs: number,
    options: ThumbnailOptions,
  ): Promise<string> {
    await mkdir(options.cacheDir, { recursive: true });
    const ffmpegBin = options.ffmpegPath ?? 'ffmpeg';
    const jpgPath = path.join(options.cacheDir, `${media.id}-${frameTimeMs}.jpg`);

    if (options.useFfmpeg !== false && media.type === 'video') {
      try {
        await access(media.path);
        const seekSec = (frameTimeMs / 1000).toFixed(3);
        await execFileAsync(ffmpegBin, [
          '-ss',
          seekSec,
          '-i',
          media.path,
          '-vframes',
          '1',
          '-q:v',
          '3',
          '-y',
          jpgPath,
        ]);
        await access(jpgPath);
        return jpgPath;
      } catch {
        // Fallback below if ffmpeg binary is absent or source file is synthetic.
      }
    }

    const svgPath = path.join(options.cacheDir, `${media.id}-${frameTimeMs}.svg`);
    const svg = buildFallbackCoverSvg(media, frameTimeMs);
    await writeFile(svgPath, svg, 'utf8');
    return svgPath;
  }

  /**
   * Enforces size-limited LRU eviction over cached thumbnail files (PRD Section 13).
   */
  async enforceLruCacheLimit(
    cacheDir: string,
    maxBytes: number,
    protectedPaths: Set<string> = new Set(),
  ): Promise<number> {
    const entries = await readdir(cacheDir);
    const fileStats = await Promise.all(
      entries.map(async (name) => {
        const full = path.join(cacheDir, name);
        const s = await stat(full);
        return { full, size: s.size, atimeMs: s.atimeMs || s.mtimeMs };
      }),
    );

    let totalBytes = fileStats.reduce((sum, item) => sum + item.size, 0);
    if (totalBytes <= maxBytes) return 0;

    fileStats.sort((a, b) => a.atimeMs - b.atimeMs);
    let evicted = 0;
    for (const item of fileStats) {
      if (totalBytes <= maxBytes) break;
      if (protectedPaths.has(item.full)) continue;
      await unlink(item.full).catch(() => {});
      totalBytes -= item.size;
      evicted += 1;
    }
    return evicted;
  }
}
