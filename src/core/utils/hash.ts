import { createHash } from 'node:crypto';
import { open, stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';

const PARTIAL_WINDOW_BYTES = 64 * 1024; // 64 KB window at start, mid, and end

/**
 * Computes streaming SHA-256 hash of a file without loading it entirely into RAM.
 */
export async function computeFileSha256(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    const stream = createReadStream(filePath, { highWaterMark: 256 * 1024 });
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
    stream.on('error', reject);
  });
}

/**
 * Computes SHA-256 of an in-memory buffer.
 */
export function computeBufferSha256(buffer: Uint8Array | Buffer): string {
  return createHash('sha256').update(buffer).digest('hex');
}

/**
 * Computes a fast partial fingerprint (size + first/middle/last 64KB chunks)
 * for instant duplicate candidate detection across 100k+ files (PRD A5).
 */
export async function computePartialHash(filePath: string): Promise<string> {
  const fileStat = await stat(filePath);
  const size = fileStat.size;
  const hash = createHash('sha256');
  hash.update(`size:${size}:`);

  if (size === 0) {
    return hash.digest('hex');
  }

  const handle = await open(filePath, 'r');
  try {
    const readWindow = async (offset: number, length: number) => {
      const buf = Buffer.allocUnsafe(length);
      const { bytesRead } = await handle.read(buf, 0, length, offset);
      hash.update(buf.subarray(0, bytesRead));
    };

    if (size <= PARTIAL_WINDOW_BYTES * 3) {
      await readWindow(0, size);
    } else {
      const midOffset = Math.max(0, Math.floor(size / 2 - PARTIAL_WINDOW_BYTES / 2));
      const endOffset = Math.max(0, size - PARTIAL_WINDOW_BYTES);
      await readWindow(0, PARTIAL_WINDOW_BYTES);
      await readWindow(midOffset, PARTIAL_WINDOW_BYTES);
      await readWindow(endOffset, PARTIAL_WINDOW_BYTES);
    }
    return hash.digest('hex');
  } finally {
    await handle.close();
  }
}
