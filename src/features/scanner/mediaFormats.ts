import type { MediaType } from '../../core/types/domain.ts';

export interface MediaFormatSpec {
  type: MediaType;
  mime: string;
}

const EXTENSION_MAP: Record<string, MediaFormatSpec> = {
  // Videos (PRD A1)
  '.mp4': { type: 'video', mime: 'video/mp4' },
  '.mkv': { type: 'video', mime: 'video/x-matroska' },
  '.webm': { type: 'video', mime: 'video/webm' },
  '.mov': { type: 'video', mime: 'video/quicktime' },
  '.avi': { type: 'video', mime: 'video/x-msvideo' },
  // Audio (PRD A1)
  '.mp3': { type: 'audio', mime: 'audio/mpeg' },
  '.m4a': { type: 'audio', mime: 'audio/mp4' },
  '.flac': { type: 'audio', mime: 'audio/flac' },
  '.wav': { type: 'audio', mime: 'audio/wav' },
  '.ogg': { type: 'audio', mime: 'audio/ogg' },
  // Images (PRD A1)
  '.png': { type: 'image', mime: 'image/png' },
  '.jpg': { type: 'image', mime: 'image/jpeg' },
  '.jpeg': { type: 'image', mime: 'image/jpeg' },
  '.gif': { type: 'image', mime: 'image/gif' },
  '.webp': { type: 'image', mime: 'image/webp' },
  '.heic': { type: 'image', mime: 'image/heic' },
};

export function classifyMediaExtension(ext: string): MediaFormatSpec | null {
  return EXTENSION_MAP[ext.toLowerCase()] ?? null;
}

export function isSupportedMediaExtension(ext: string): boolean {
  return Boolean(classifyMediaExtension(ext));
}
