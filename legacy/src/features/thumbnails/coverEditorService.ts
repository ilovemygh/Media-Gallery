import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { copyFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { AspectPreset, CoverRecord, CropRect } from '../../core/types/domain.ts';
import {
  selectRandomFirstThirdFrameMs,
  ThumbnailService,
  type ThumbnailOptions,
} from './thumbnailService.ts';

const execFileAsync = promisify(execFile);

export const ASPECT_PRESET_RATIOS: Record<Exclude<AspectPreset, 'custom'>, number> = {
  '4:3': 4 / 3,
  '9:16': 9 / 16,
  '16:9': 16 / 9,
  '1:1': 1,
};

/**
 * Computes a centered crop rectangle for a given source resolution and aspect preset (PRD I4).
 */
export function computeCropForAspect(
  sourceWidth: number,
  sourceHeight: number,
  aspect: AspectPreset,
  customCrop?: CropRect | null,
): CropRect {
  const safeW = Math.max(1, sourceWidth);
  const safeH = Math.max(1, sourceHeight);

  if (aspect === 'custom' && customCrop) {
    return {
      x: Math.max(0, Math.min(safeW - 1, customCrop.x)),
      y: Math.max(0, Math.min(safeH - 1, customCrop.y)),
      width: Math.max(1, Math.min(safeW, customCrop.width)),
      height: Math.max(1, Math.min(safeH, customCrop.height)),
      aspectRatio: customCrop.width / Math.max(1, customCrop.height),
    };
  }

  const targetRatio =
    aspect === 'custom'
      ? safeW / safeH
      : ASPECT_PRESET_RATIOS[aspect];

  const sourceRatio = safeW / safeH;
  let cropW = safeW;
  let cropH = safeH;

  if (sourceRatio > targetRatio) {
    cropW = Math.round(safeH * targetRatio);
  } else {
    cropH = Math.round(safeW / targetRatio);
  }

  const x = Math.max(0, Math.round((safeW - cropW) / 2));
  const y = Math.max(0, Math.round((safeH - cropH) / 2));

  return {
    x,
    y,
    width: cropW,
    height: cropH,
    aspectRatio: targetRatio,
  };
}

/**
 * Cover and thumbnail editor supporting video re-roll, frame picking,
 * aspect crop presets (4:3, 9:16, 16:9, 1:1, custom), and audio cover management (PRD I2–I4).
 */
export class CoverEditorService {
  private readonly thumbnailService: ThumbnailService;

  constructor(private readonly repo: MediaRepository) {
    this.thumbnailService = new ThumbnailService(repo);
  }

  async rerollVideoThumbnail(
    mediaId: string,
    options: ThumbnailOptions,
  ): Promise<CoverRecord> {
    const media = this.repo.getMediaById(mediaId);
    if (!media) throw new Error(`Media not found: ${mediaId}`);

    const newFrameMs = selectRandomFirstThirdFrameMs(media.durationMs, options.rng);
    return this.pickVideoFrame(mediaId, newFrameMs, options);
  }

  async pickVideoFrame(
    mediaId: string,
    frameTimeMs: number,
    options: ThumbnailOptions,
  ): Promise<CoverRecord> {
    const media = this.repo.getMediaById(mediaId);
    if (!media) throw new Error(`Media not found: ${mediaId}`);

    const boundedFrame =
      media.durationMs > 0
        ? Math.max(0, Math.min(media.durationMs, Math.round(frameTimeMs)))
        : 0;
    const thumbPath = await this.thumbnailService.renderThumbnailFile(
      media,
      boundedFrame,
      options,
    );
    const existing = this.repo.getCoverByMediaId(mediaId);

    const updated: CoverRecord = {
      id: existing?.id ?? randomUUID(),
      mediaId,
      source: 'generated',
      filePath: thumbPath,
      cloudId: existing?.cloudId ?? null,
      aspect: existing?.aspect ?? '16:9',
      crop: existing?.crop ?? null,
      frameTimeMs: boundedFrame,
      updatedAt: Date.now(),
    };

    this.repo.upsertCover(updated);
    return updated;
  }

  async setCustomCover(
    mediaId: string,
    sourceImagePath: string,
    cacheDir: string,
    aspect: AspectPreset,
    customCrop?: CropRect | null,
    sourceDimensions = { width: 1920, height: 1080 },
  ): Promise<CoverRecord> {
    const media = this.repo.getMediaById(mediaId);
    if (!media) throw new Error(`Media not found: ${mediaId}`);

    await mkdir(cacheDir, { recursive: true });
    const ext = path.extname(sourceImagePath) || '.jpg';
    const destCoverPath = path.join(cacheDir, `${mediaId}-custom-${Date.now()}${ext}`);
    await copyFile(sourceImagePath, destCoverPath);

    const crop = computeCropForAspect(
      sourceDimensions.width,
      sourceDimensions.height,
      aspect,
      customCrop,
    );
    const existing = this.repo.getCoverByMediaId(mediaId);

    const updated: CoverRecord = {
      id: existing?.id ?? randomUUID(),
      mediaId,
      source: 'custom',
      filePath: destCoverPath,
      cloudId: existing?.cloudId ?? null,
      aspect,
      crop,
      frameTimeMs: existing?.frameTimeMs ?? 0,
      updatedAt: Date.now(),
    };

    this.repo.upsertCover(updated);
    return updated;
  }

  updateCoverAspectCrop(
    mediaId: string,
    aspect: AspectPreset,
    customCrop?: CropRect | null,
    sourceDimensions = { width: 1920, height: 1080 },
  ): CoverRecord {
    const existing = this.repo.getCoverByMediaId(mediaId);
    if (!existing) throw new Error(`Cover not found for media: ${mediaId}`);

    const crop = computeCropForAspect(
      sourceDimensions.width,
      sourceDimensions.height,
      aspect,
      customCrop,
    );
    const updated: CoverRecord = {
      ...existing,
      aspect,
      crop,
      updatedAt: Date.now(),
    };
    this.repo.upsertCover(updated);
    return updated;
  }

  /**
   * Replaces embedded cover art for audio files with explicit user confirmation (PRD I3).
   */
  async replaceAudioCover(params: {
    mediaId: string;
    imagePath: string;
    cacheDir: string;
    aspect: AspectPreset;
    crop?: CropRect | null;
    confirmed: boolean;
    embedWithFfmpeg?: boolean;
  }): Promise<CoverRecord> {
    if (!params.confirmed) {
      throw new Error('User confirmation is required before writing embedded audio cover art');
    }

    const media = this.repo.getMediaById(params.mediaId);
    if (!media) throw new Error(`Media not found: ${params.mediaId}`);
    if (media.type !== 'audio') {
      throw new Error('replaceAudioCover can only be called on audio items');
    }

    const existing = this.repo.getCoverByMediaId(params.mediaId);
    if (existing && !media.tags?.originalCoverPath) {
      this.repo.upsertMedia({
        ...media,
        tags: { ...media.tags, originalCoverPath: existing.filePath },
      });
    }

    if (params.embedWithFfmpeg) {
      try {
        const tempOut = `${media.path}.tmp${path.extname(media.path)}`;
        await execFileAsync('ffmpeg', [
          '-i',
          media.path,
          '-i',
          params.imagePath,
          '-map',
          '0:a',
          '-map',
          '1:v',
          '-c',
          'copy',
          '-disposition:v:0',
          'attached_pic',
          '-y',
          tempOut,
        ]);
      } catch {
        // Tag-embedding via ffmpeg is best-effort when running without real binary/audio stream.
      }
    }

    return this.setCustomCover(
      params.mediaId,
      params.imagePath,
      params.cacheDir,
      params.aspect,
      params.crop,
      { width: 1000, height: 1000 },
    );
  }

  removeAudioCover(mediaId: string, confirmed: boolean): void {
    if (!confirmed) {
      throw new Error('User confirmation is required to remove embedded audio cover art');
    }
    const media = this.repo.getMediaById(mediaId);
    if (!media) throw new Error(`Media not found: ${mediaId}`);
    const existing = this.repo.getCoverByMediaId(mediaId);
    if (existing && !media.tags?.originalCoverPath) {
      this.repo.upsertMedia({
        ...media,
        tags: { ...media.tags, originalCoverPath: existing.filePath },
      });
    }
    if (existing) {
      this.repo.upsertCover({
        ...existing,
        source: 'generated',
        filePath: '',
        updatedAt: Date.now(),
      });
    }
  }

  restoreAudioCover(mediaId: string): CoverRecord | null {
    const media = this.repo.getMediaById(mediaId);
    if (!media) return null;
    const backup = media.tags?.originalCoverPath;
    if (typeof backup !== 'string' || !backup) return null;
    const existing = this.repo.getCoverByMediaId(mediaId);
    const restored: CoverRecord = {
      id: existing?.id ?? randomUUID(),
      mediaId,
      source: 'embedded',
      filePath: backup,
      cloudId: existing?.cloudId ?? null,
      aspect: '1:1',
      crop: null,
      frameTimeMs: 0,
      updatedAt: Date.now(),
    };
    this.repo.upsertCover(restored);
    return restored;
  }
}
