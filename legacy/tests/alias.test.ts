import { describe, expect, it } from 'vitest';
import { cleanFileNameForAlias, generateAutoAlias } from '../src/features/alias/aliasEngine.ts';

describe('Auto-Alias Rules (PRD Section 9)', () => {
  it('strips camera prefixes, replaces separators, splits camelCase, and trims isolated numbers', () => {
    expect(cleanFileNameForAlias('IMG_2031.jpg')).toBe('');
    expect(cleanFileNameForAlias('VID_0042_sunsetWalk-09.mp4')).toBe('sunset Walk');
    expect(cleanFileNameForAlias('Screenshot_myHolidayTrip_2026.png')).toBe('my Holiday Trip');
  });

  it('generates {date} {place} {name} using offline reverse geocoding when GPS is present', () => {
    const march2026 = Date.UTC(2026, 2, 14, 10, 0, 0);
    const result = generateAutoAlias({
      fileName: 'IMG_2031.jpg',
      type: 'image',
      createdAt: march2026,
      modifiedAt: march2026,
      gpsLat: -8.7185,
      gpsLon: 115.1686,
      deviceName: 'Pixel 9',
    });

    expect(result.resolvedPlace).toBe('Kuta Beach Bali');
    expect(result.autoAlias).toBe('2026-03-14 Kuta Beach Bali');
  });

  it('uses Artist - Title for tagged audio and video title for YouTube videos', () => {
    const now = Date.UTC(2026, 4, 1, 12, 0, 0);
    const audioAlias = generateAutoAlias({
      fileName: 'track_09.mp3',
      type: 'audio',
      createdAt: now,
      modifiedAt: now,
      tags: { artist: 'Nova Drift', title: 'Midnight Echoes' },
    });
    expect(audioAlias.autoAlias).toBe('Nova Drift - Midnight Echoes');

    const ytAlias = generateAutoAlias({
      fileName: 'yt_dQw4w9.mp4',
      type: 'video',
      createdAt: now,
      modifiedAt: now,
      tags: { source: 'youtube', title: 'Cross Platform Engineering Talk' },
    });
    expect(ytAlias.autoAlias).toBe('Cross Platform Engineering Talk');
  });

  it('supports custom alias templates including {device} and {time}', () => {
    const evening = new Date(2026, 2, 14, 19, 30, 0).getTime();
    const res = generateAutoAlias({
      fileName: 'VID_beachWalk.mp4',
      type: 'video',
      createdAt: evening,
      modifiedAt: evening,
      deviceName: 'Laptop',
      template: '{date} {time} {device} {name}',
      useUtcDate: false,
    });
    expect(res.autoAlias).toBe('2026-03-14 evening Laptop beach Walk');
  });
});
