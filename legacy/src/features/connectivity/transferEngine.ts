import { randomUUID } from 'node:crypto';
import { access, mkdir, open, rename, stat } from 'node:fs/promises';
import path from 'node:path';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { DeviceRecord, MediaRecord, TransferRecord } from '../../core/types/domain.ts';
import { computeFileSha256 } from '../../core/utils/hash.ts';
import type { PairingService } from './pairingService.ts';
import type { TransferChunkPacket, TransportChannel } from './transportInterface.ts';

export const DEFAULT_CHUNK_SIZE = 256 * 1024; // 256 KB

export interface ReceiverSinkOptions {
  destinationDir: string;
}

/**
 * Receiver-side chunk assembler that writes chunks at exact byte offsets
 * into `.part` staging files and verifies full SHA-256 before committing.
 */
export class TransferReceiver {
  private readonly completedPaths = new Map<string, string>();

  constructor(private readonly destinationDir: string) {}

  getPartFilePath(transferId: string, fileName: string): string {
    return path.join(this.destinationDir, `.part-${transferId}-${path.basename(fileName)}`);
  }

  getFinalFilePath(fileName: string): string {
    return path.join(this.destinationDir, path.basename(fileName));
  }

  getCompletedFilePath(transferId: string): string | undefined {
    return this.completedPaths.get(transferId);
  }

  async handleIncomingChunk(packet: TransferChunkPacket): Promise<{ completed: boolean; finalPath?: string }> {
    await mkdir(this.destinationDir, { recursive: true });
    const partPath = this.getPartFilePath(packet.transferId, packet.fileName);

    let fileExists = false;
    try {
      await access(partPath);
      fileExists = true;
    } catch {
      fileExists = false;
    }

    const handle = await open(partPath, fileExists ? 'r+' : 'w+');
    try {
      if (packet.data.byteLength > 0) {
        await handle.write(packet.data, 0, packet.data.byteLength, packet.offset);
      }
    } finally {
      await handle.close();
    }

    const partStat = await stat(partPath);
    if (partStat.size >= packet.bytesTotal && packet.chunkIndex + 1 >= packet.totalChunks) {
      const actualHash = await computeFileSha256(partPath);
      if (actualHash !== packet.expectedHash) {
        throw new Error(
          `SHA-256 verification failed for ${packet.fileName}: expected ${packet.expectedHash}, got ${actualHash}`,
        );
      }
      const finalPath = this.getFinalFilePath(packet.fileName);
      await rename(partPath, finalPath);
      this.completedPaths.set(packet.transferId, finalPath);
      return { completed: true, finalPath };
    }

    return { completed: false };
  }
}

export interface SendFileOptions {
  media: MediaRecord;
  fromDeviceId: string;
  toDevice: DeviceRecord;
  transport: TransportChannel;
  chunkSize?: number;
  maxChunksPerRun?: number; // Allows tests/pauses to simulate mid-transfer interruption cleanly
  skipConfirmationCheck?: boolean;
  onProgress?: (record: TransferRecord) => void;
}

/**
 * Chunked, resumable, SHA-256 verified transfer engine (PRD D7, 10.2, 10.5).
 */
export class TransferEngine {
  private readonly pausedOrCancelled = new Map<string, 'paused' | 'cancelled'>();

  constructor(
    private readonly repo: MediaRepository,
    private readonly pairingService?: PairingService,
  ) {}

  pauseTransfer(transferId: string): void {
    this.pausedOrCancelled.set(transferId, 'paused');
    const existing = this.repo.getTransfer(transferId);
    if (existing) {
      this.repo.upsertTransfer({ ...existing, status: 'paused', updatedAt: Date.now() });
    }
  }

  cancelTransfer(transferId: string): void {
    this.pausedOrCancelled.set(transferId, 'cancelled');
    const existing = this.repo.getTransfer(transferId);
    if (existing) {
      this.repo.upsertTransfer({ ...existing, status: 'cancelled', updatedAt: Date.now() });
    }
  }

  async sendFile(options: SendFileOptions): Promise<TransferRecord> {
    if (
      !options.skipConfirmationCheck &&
      this.pairingService &&
      !this.pairingService.isPaired(options.toDevice.id)
    ) {
      throw new Error(
        `User confirmation required before transferring to unpaired device "${options.toDevice.name}"`,
      );
    }

    const fileStat = await stat(options.media.path);
    const totalBytes = fileStat.size;
    const expectedHash = await computeFileSha256(options.media.path);
    const chunkSize = options.chunkSize ?? DEFAULT_CHUNK_SIZE;

    const record: TransferRecord = {
      id: randomUUID(),
      mediaId: options.media.id,
      fileName: options.media.fileName,
      fromDevice: options.fromDeviceId,
      toDevice: options.toDevice.id,
      status: 'transferring',
      bytesDone: 0,
      bytesTotal: totalBytes,
      lastChunkIndex: 0,
      chunkSize,
      transport: options.transport.kind,
      speedBps: 0,
      expectedHash,
      error: null,
      updatedAt: Date.now(),
    };

    this.repo.upsertTransfer(record);
    return this.streamRemainingChunks(record, options.media.path, options.toDevice, options.transport, options);
  }

  async resumeTransfer(params: {
    transferId: string;
    sourceFilePath: string;
    toDevice: DeviceRecord;
    transport: TransportChannel;
    onProgress?: (record: TransferRecord) => void;
  }): Promise<TransferRecord> {
    const existing = this.repo.getTransfer(params.transferId);
    if (!existing) {
      throw new Error(`Transfer record not found: ${params.transferId}`);
    }
    this.pausedOrCancelled.delete(params.transferId);
    const resumed: TransferRecord = {
      ...existing,
      status: 'transferring',
      transport: params.transport.kind,
      error: null,
      updatedAt: Date.now(),
    };
    this.repo.upsertTransfer(resumed);

    return this.streamRemainingChunks(
      resumed,
      params.sourceFilePath,
      params.toDevice,
      params.transport,
      { onProgress: params.onProgress },
    );
  }

  private async streamRemainingChunks(
    record: TransferRecord,
    sourceFilePath: string,
    toDevice: DeviceRecord,
    transport: TransportChannel,
    opts: { maxChunksPerRun?: number; onProgress?: (record: TransferRecord) => void },
  ): Promise<TransferRecord> {
    const totalChunks = Math.max(1, Math.ceil(record.bytesTotal / record.chunkSize));
    const handle = await open(sourceFilePath, 'r');
    let chunksSentThisRun = 0;
    const startedAt = Date.now();
    const initialBytesDone = record.bytesDone;

    try {
      let current = { ...record };

      while (current.lastChunkIndex < totalChunks) {
        const interruptedState = this.pausedOrCancelled.get(current.id);
        if (interruptedState) {
          current = { ...current, status: interruptedState, updatedAt: Date.now() };
          this.repo.upsertTransfer(current);
          return current;
        }

        if (opts.maxChunksPerRun != null && chunksSentThisRun >= opts.maxChunksPerRun) {
          current = { ...current, status: 'paused', updatedAt: Date.now() };
          this.repo.upsertTransfer(current);
          return current;
        }

        const chunkIndex = current.lastChunkIndex;
        const offset = chunkIndex * current.chunkSize;
        const length = Math.min(current.chunkSize, Math.max(0, current.bytesTotal - offset));
        const buffer = Buffer.allocUnsafe(length);
        const { bytesRead } = length > 0 ? await handle.read(buffer, 0, length, offset) : { bytesRead: 0 };

        await transport.sendChunk(toDevice, {
          transferId: current.id,
          mediaId: current.mediaId,
          fileName: current.fileName,
          chunkIndex,
          totalChunks,
          offset,
          bytesTotal: current.bytesTotal,
          expectedHash: current.expectedHash,
          data: buffer.subarray(0, bytesRead),
        });

        chunksSentThisRun += 1;
        const elapsedSec = Math.max(0.001, (Date.now() - startedAt) / 1000);
        const newBytesDone = Math.min(current.bytesTotal, offset + bytesRead);
        current = {
          ...current,
          bytesDone: newBytesDone,
          lastChunkIndex: chunkIndex + 1,
          speedBps: Math.round((newBytesDone - initialBytesDone) / elapsedSec),
          status: chunkIndex + 1 >= totalChunks ? 'completed' : 'transferring',
          updatedAt: Date.now(),
        };
        this.repo.upsertTransfer(current);
        opts.onProgress?.(current);
      }

      return current;
    } catch (error) {
      const errored: TransferRecord = {
        ...record,
        status: 'error',
        error: error instanceof Error ? error.message : String(error),
        updatedAt: Date.now(),
      };
      this.repo.upsertTransfer(errored);
      return errored;
    } finally {
      await handle.close();
    }
  }
}
