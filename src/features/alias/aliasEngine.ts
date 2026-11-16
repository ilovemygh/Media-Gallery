import type { MediaTags, MediaType } from '../../core/types/domain.ts';
import { lookupPlaceOffline } from './offlineGeocoder.ts';

const KNOWN_PREFIXES = /^(?:IMG|VID|Screenshot|PXL|DCIM|MVIMG|WIN|AUD|REC)[_-]+/i;

/**
 * Formats epoch ms into YYYY-MM-DD using UTC or local date consistently.
 */
export function formatDateToken(timestampMs: number, useUtc = true): string {
  const date = new Date(timestampMs);
  const year = useUtc ? date.getUTCFullYear() : date.getFullYear();
  const month = String((useUtc ? date.getUTCMonth() : date.getMonth()) + 1).padStart(2, '0');
  const day = String(useUtc ? date.getUTCDate() : date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function formatTimePeriodToken(timestampMs: number, useUtc = false): string {
  const date = new Date(timestampMs);
  const hour = useUtc ? date.getUTCHours() : date.getHours();
  if (hour >= 5 && hour < 12) return 'morning';
  if (hour >= 12 && hour < 17) return 'afternoon';
  if (hour >= 17 && hour < 21) return 'evening';
  return 'night';
}

/**
 * Cleans a raw filename per PRD Section 9:
 * - Strip extension
 * - Remove prefixes (`IMG_`, `VID_`, `Screenshot_`, etc.)
 * - Replace `_` and `-` with spaces
 * - Split `camelCase` (`beachTrip` -> `beach Trip`)
 * - Trim standalone numeric tokens (e.g. `2031`, `0042`)
 */
export function cleanFileNameForAlias(fileName: string): string {
  const lastSlash = Math.max(fileName.lastIndexOf('/'), fileName.lastIndexOf('\\'));
  const base = lastSlash >= 0 ? fileName.slice(lastSlash + 1) : fileName;
  const dotIdx = base.lastIndexOf('.');
  const stem = dotIdx > 0 ? base.slice(0, dotIdx) : base;

  const withoutPrefix = stem.replace(KNOWN_PREFIXES, '');
  const spacedDelimiters = withoutPrefix.replace(/[_-]+/g, ' ');
  const splitCamel = spacedDelimiters
    .replace(/([a-z\d])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2');

  const trimmedNumbers = splitCamel
    .replace(/\b\d+\b/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  return trimmedNumbers;
}

export interface AutoAliasInput {
  fileName: string;
  type: MediaType;
  createdAt: number;
  modifiedAt: number;
  gpsLat?: number | null;
  gpsLon?: number | null;
  place?: string | null;
  tags?: MediaTags;
  deviceName?: string;
  template?: string;
  onlineGeocodingEnabled?: boolean;
  useUtcDate?: boolean;
}

/**
 * Generates an automatic suggested alias following PRD Section 9 rules.
 */
export function generateAutoAlias(input: AutoAliasInput): {
  autoAlias: string;
  resolvedPlace: string | null;
} {
  const timestamp = input.createdAt > 0 ? input.createdAt : input.modifiedAt;
  const dateToken = formatDateToken(timestamp, input.useUtcDate ?? true);
  const timeToken = formatTimePeriodToken(timestamp);

  const resolvedPlace =
    (input.place && input.place.trim()) ||
    lookupPlaceOffline(input.gpsLat, input.gpsLon) ||
    null;

  // Audio and YouTube video tag overrides per PRD Section 9
  let nameToken = '';
  const tags = input.tags ?? {};
  if (input.type === 'audio' && (tags.artist || tags.title)) {
    nameToken = tags.artist && tags.title
      ? `${tags.artist} - ${tags.title}`
      : String(tags.title ?? tags.artist);
  } else if (
    input.type === 'video' &&
    (tags.source === 'youtube' || Boolean(tags.youtubeId)) &&
    tags.title
  ) {
    nameToken = String(tags.title);
  } else {
    nameToken = cleanFileNameForAlias(input.fileName);
  }

  const template = input.template?.trim() || '{date} {place} {name}';

  // Audio tracks and YouTube videos with explicit tags default directly to tag format
  // when using the default template and no GPS place is attached, unless overridden.
  const hasExplicitAudioTag =
    input.type === 'audio' && Boolean(tags.artist && tags.title) && template === '{date} {place} {name}';
  const hasExplicitYouTubeTitle =
    input.type === 'video' &&
    Boolean((tags.source === 'youtube' || tags.youtubeId) && tags.title) &&
    template === '{date} {place} {name}';

  let rendered: string;
  if (hasExplicitAudioTag || hasExplicitYouTubeTitle) {
    rendered = nameToken;
  } else {
    rendered = template
      .replace(/\{date\}/g, dateToken)
      .replace(/\{time\}/g, timeToken)
      .replace(/\{place\}/g, resolvedPlace ?? '')
      .replace(/\{name\}/g, nameToken)
      .replace(/\{device\}/g, input.deviceName ?? '')
      .replace(/\{artist\}/g, String(tags.artist ?? ''))
      .replace(/\{title\}/g, String(tags.title ?? ''))
      .replace(/\s+/g, ' ')
      .trim();
  }

  if (!rendered) {
    const fallbackStem = input.fileName.replace(/\.[^.]+$/, '');
    rendered = fallbackStem;
  }

  return {
    autoAlias: rendered,
    resolvedPlace,
  };
}
