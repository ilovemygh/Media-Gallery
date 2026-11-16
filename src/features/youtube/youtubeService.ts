import { execFile, spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, readdir } from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';
import type { MediaRepository } from '../../core/database/repository.ts';
import type {
  DownloadFormat,
  DownloadJobRecord,
  MediaRecord,
} from '../../core/types/domain.ts';
import { ScannerService } from '../scanner/scannerService.ts';

const execFileAsync = promisify(execFile);

export const YOUTUBE_COMPLIANCE_NOTICE =
  'Download only content you have the right to download under applicable laws and platform terms.';

export const YOUTUBE_PORTAL_URLS = {
  upload: 'https://studio.youtube.com/channel/UC/videos/upload',
  channel: 'https://www.youtube.com/channel/mine',
  studio: 'https://studio.youtube.com',
} as const;

export interface YoutubeDownloadRequest {
  url: string;
  format: DownloadFormat;
  quality: string; // e.g. '1080p', '720p', 'best', '320k', '192k'
  destinationDeviceId: string;
  destinationFolder: string;
  allowPlaylist?: boolean;
  ytDlpBin?: string;
  ffmpegBin?: string;
}

export interface ProcessRunner {
  runDownload(
    bin: string,
    args: string[],
    onProgress: (percent: number, title?: string) => void,
  ): Promise<{ outputPaths: string[]; title?: string; uploader?: string }>;
  runUpdate(bin: string): Promise<string>;
}

/**
 * Builds deterministic `yt-dlp` CLI arguments for MP4 video or MP3 audio + embedded cover/metadata.
 */
export function buildYtDlpArgs(req: YoutubeDownloadRequest): string[] {
  const outputTemplate = path.join(req.destinationFolder, '%(title)s [%(id)s].%(ext)s');
  const args: string[] = [
    '--newline',
    '--write-thumbnail',
    '--embed-thumbnail',
    '--add-metadata',
    '--output',
    outputTemplate,
  ];

  if (req.allowPlaylist) {
    args.push('--yes-playlist');
  } else {
    args.push('--no-playlist');
  }

  if (req.ffmpegBin) {
    args.push('--ffmpeg-location', req.ffmpegBin);
  }

  if (req.format === 'mp3') {
    const audioQ = req.quality.replace(/[^0-9kK]/g, '') || '192K';
    args.push(
      '--extract-audio',
      '--audio-format',
      'mp3',
      '--audio-quality',
      audioQ,
    );
  } else {
    const maxHeightMatch = req.quality.match(/(\d{3,4})p/i);
    const formatExpr = maxHeightMatch
      ? `bestvideo[height<=${maxHeightMatch[1]}]+bestaudio/best[height<=${maxHeightMatch[1]}]`
      : 'bestvideo+bestaudio/best';
    args.push('-f', formatExpr, '--merge-output-format', 'mp4');
  }

  args.push(req.url);
  return args;
}

const defaultProcessRunner: ProcessRunner = {
  async runDownload(bin, args, onProgress) {
    return new Promise((resolve, reject) => {
      const child = spawn(bin, args, { stdio: ['ignore', 'pipe', 'pipe'] });
      const createdFiles = new Set<string>();
      let stderr = '';
      let detectedTitle: string | undefined;

      const parseLine = (line: string) => {
        const pctMatch = line.match(/\[download\]\s+(\d+(?:\.\d+)?)%/);
        if (pctMatch) {
          onProgress(Number(pctMatch[1]), detectedTitle);
        }
        const destMatch = line.match(/(?:Destination|Merging formats into|\[ExtractAudio\] Destination):\s+"?(.+?)"?$/);
        if (destMatch) {
          createdFiles.add(destMatch[1].trim());
          detectedTitle = path.basename(destMatch[1].trim(), path.extname(destMatch[1].trim()));
        }
      };

      child.stdout.on('data', (chunk: Buffer) => {
        chunk.toString('utf8').split(/\r?\n/).forEach(parseLine);
      });
      child.stderr.on('data', (chunk: Buffer) => {
        const txt = chunk.toString('utf8');
        stderr += txt;
      });
      child.on('error', reject);
      child.on('close', (code) => {
        if (code === 0) {
          resolve({ outputPaths: Array.from(createdFiles), title: detectedTitle });
        } else {
          reject(new Error(`yt-dlp exited with code ${code}: ${stderr.slice(0, 400)}`));
        }
      });
    });
  },
  async runUpdate(bin) {
    const { stdout, stderr } = await execFileAsync(bin, ['-U']);
    return `${stdout}${stderr}`.trim();
  },
};

/**
 * YouTube download queue, metadata tagger, library auto-importer, and yt-dlp updater (PRD G1–G6, F5).
 */
export class YoutubeService {
  private readonly scanner: ScannerService;

  constructor(
    private readonly repo: MediaRepository,
    private readonly runner: ProcessRunner = defaultProcessRunner,
  ) {
    this.scanner = new ScannerService(repo);
  }

  async updateYtDlp(ytDlpBin = 'yt-dlp'): Promise<string> {
    return this.runner.runUpdate(ytDlpBin);
  }

  async enqueueAndDownload(
    req: YoutubeDownloadRequest,
    options: { deviceName: string; cacheDir?: string; onJobUpdate?: (job: DownloadJobRecord) => void },
  ): Promise<{ job: DownloadJobRecord; importedMedia: MediaRecord[] }> {
    await mkdir(req.destinationFolder, { recursive: true });
    const now = Date.now();

    let job: DownloadJobRecord = {
      id: randomUUID(),
      url: req.url,
      title: null,
      format: req.format,
      quality: req.quality,
      destinationDeviceId: req.destinationDeviceId,
      destinationFolder: req.destinationFolder.replace(/\\/g, '/'),
      status: 'downloading',
      progress: 0,
      outputMediaId: null,
      error: null,
      createdAt: now,
      updatedAt: now,
    };
    this.repo.upsertDownloadJob(job);
    options.onJobUpdate?.(job);

    try {
      const beforeFiles = new Set(await readdir(req.destinationFolder));
      const args = buildYtDlpArgs(req);
      const bin = req.ytDlpBin ?? 'yt-dlp';

      const result = await this.runner.runDownload(bin, args, (percent, title) => {
        job = {
          ...job,
          progress: Math.min(99, percent),
          title: title ?? job.title,
          updatedAt: Date.now(),
        };
        this.repo.upsertDownloadJob(job);
        options.onJobUpdate?.(job);
      });

      let targetPaths = result.outputPaths;
      if (targetPaths.length === 0) {
        const afterFiles = await readdir(req.destinationFolder);
        targetPaths = afterFiles
          .filter((name) => !beforeFiles.has(name))
          .map((name) => path.join(req.destinationFolder, name));
      }

      const importedMedia: MediaRecord[] = [];
      for (const filePath of targetPaths) {
        const expectedExt = req.format === 'mp3' ? '.mp3' : '.mp4';
        if (path.extname(filePath).toLowerCase() !== expectedExt) continue;

        const titleStem = result.title ?? path.basename(filePath, expectedExt);
        const record = await this.scanner.indexSingleFile(filePath, {
          deviceId: req.destinationDeviceId,
          deviceName: options.deviceName,
          cacheDir: options.cacheDir,
          generateThumbnails: Boolean(options.cacheDir),
          probeMetadata: async () => ({
            tags: {
              source: 'youtube',
              title: titleStem,
              uploader: result.uploader ?? 'YouTube',
              artist: req.format === 'mp3' ? (result.uploader ?? 'YouTube') : undefined,
            },
          }),
        });
        if (record) importedMedia.push(record);
      }

      job = {
        ...job,
        status: 'completed',
        progress: 100,
        title: result.title ?? importedMedia[0]?.alias ?? job.title,
        outputMediaId: importedMedia[0]?.id ?? null,
        updatedAt: Date.now(),
      };
      this.repo.upsertDownloadJob(job);
      options.onJobUpdate?.(job);

      return { job, importedMedia };
    } catch (error) {
      job = {
        ...job,
        status: 'error',
        error: error instanceof Error ? error.message : String(error),
        updatedAt: Date.now(),
      };
      this.repo.upsertDownloadJob(job);
      options.onJobUpdate?.(job);
      throw error;
    }
  }
}
