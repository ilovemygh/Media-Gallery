/**
 * Offline-first privacy-preserving reverse geocoder (PRD Section 9 & Section 15).
 * Resolves GPS latitude/longitude to a human-readable place name using a local
 * gazetteer without any network requests unless the user opts into online lookup.
 */

interface GazetteerEntry {
  name: string;
  lat: number;
  lon: number;
  radiusKm: number;
}

const OFFLINE_GAZETTEER: GazetteerEntry[] = [
  { name: 'Kuta Beach Bali', lat: -8.7185, lon: 115.1686, radiusKm: 35 },
  { name: 'Ubud Bali', lat: -8.5069, lon: 115.2625, radiusKm: 30 },
  { name: 'Jakarta', lat: -6.2088, lon: 106.8456, radiusKm: 60 },
  { name: 'Ancol Beach Jakarta', lat: -6.1256, lon: 106.842, radiusKm: 15 },
  { name: 'Singapore', lat: 1.3521, lon: 103.8198, radiusKm: 35 },
  { name: 'Tokyo', lat: 35.6762, lon: 139.6503, radiusKm: 65 },
  { name: 'Santa Monica Beach', lat: 34.0195, lon: -118.4912, radiusKm: 35 },
  { name: 'San Francisco', lat: 37.7749, lon: -122.4194, radiusKm: 45 },
  { name: 'New York', lat: 40.7128, lon: -74.006, radiusKm: 55 },
  { name: 'London', lat: 51.5072, lon: -0.1276, radiusKm: 55 },
  { name: 'Paris', lat: 48.8566, lon: 2.3522, radiusKm: 50 },
  { name: 'Bondi Beach Sydney', lat: -33.8915, lon: 151.2767, radiusKm: 35 },
];

function haversineKm(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const toRad = (deg: number) => (deg * Math.PI) / 180;
  const r = 6371;
  const dLat = toRad(lat2 - lat1);
  const dLon = toRad(lon2 - lon1);
  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(toRad(lat1)) * Math.cos(toRad(lat2)) * Math.sin(dLon / 2) * Math.sin(dLon / 2);
  return r * 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
}

export function lookupPlaceOffline(
  lat: number | null | undefined,
  lon: number | null | undefined,
): string | null {
  if (lat == null || lon == null || !Number.isFinite(lat) || !Number.isFinite(lon)) {
    return null;
  }

  let bestMatch: { name: string; distance: number } | null = null;
  for (const entry of OFFLINE_GAZETTEER) {
    const distance = haversineKm(lat, lon, entry.lat, entry.lon);
    if (distance <= entry.radiusKm && (!bestMatch || distance < bestMatch.distance)) {
      bestMatch = { name: entry.name, distance };
    }
  }
  return bestMatch ? bestMatch.name : null;
}
