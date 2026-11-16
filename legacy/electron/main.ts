import os from 'node:os';
import path from 'node:path';
import { app, BrowserWindow, ipcMain, shell } from 'electron';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import {
  generateDeviceIdentityKeys,
  PairingService,
} from '../src/features/connectivity/pairingService.ts';
import { WifiLanTransport } from '../src/features/connectivity/wifiLanTransport.ts';
import { LibraryFileWatcher } from '../src/features/scanner/fileWatcher.ts';
import { ScannerService } from '../src/features/scanner/scannerService.ts';
import { YoutubeService, type YoutubeDownloadRequest } from '../src/features/youtube/youtubeService.ts';

let mainWindow: BrowserWindow | null = null;
let repo: MediaRepository | null = null;
let watcher: LibraryFileWatcher | null = null;
let lanTransport: WifiLanTransport | null = null;

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1380,
    height: 860,
    minWidth: 960,
    minHeight: 620,
    title: 'Media Gallery',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    void mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    void mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }
}

async function bootstrapDesktopServices(): Promise<void> {
  const userDataDir = app.getPath('userData');
  const dbPath = path.join(userDataDir, 'media-gallery.db');
  const thumbCacheDir = path.join(userDataDir, 'thumbnails');

  repo = new MediaRepository(new NodeSqliteAdapter(dbPath));
  const keys = generateDeviceIdentityKeys();
  const selfDeviceId = `pc-${os.hostname().toLowerCase()}`;
  const selfDevice = {
    id: selfDeviceId,
    name: os.hostname(),
    type: 'pc' as const,
    publicKey: keys.fingerprint,
    lastSeen: Date.now(),
    transports: ['wired', 'wifi_lan', 'bluetooth'] as const,
    host: '127.0.0.1',
    port: 9443,
    isSelf: true,
    isOnline: true,
  };
  repo.upsertDevice({ ...selfDevice, transports: [...selfDevice.transports] });

  const scanner = new ScannerService(repo);
  const youtube = new YoutubeService(repo);
  const pairing = new PairingService(repo);
  lanTransport = new WifiLanTransport();
  await lanTransport.startServer(9443, {
    onChunkReceived: async () => {},
  });
  lanTransport.startMdnsAnnouncer({ ...selfDevice, transports: [...selfDevice.transports] });

  watcher = new LibraryFileWatcher(repo, {
    deviceId: selfDeviceId,
    deviceName: selfDevice.name,
    cacheDir: thumbCacheDir,
    generateThumbnails: true,
  });

  ipcMain.handle('mg:scan-folder', async (_event, folderPath: string) => {
    watcher?.start([folderPath]);
    return scanner.scanFolders({
      deviceId: selfDeviceId,
      deviceName: selfDevice.name,
      roots: [folderPath],
      cacheDir: thumbCacheDir,
      generateThumbnails: true,
    });
  });

  ipcMain.handle('mg:query-media', (_event, query: string, filters: Record<string, unknown>) => {
    return repo?.queryMedia(query, filters) ?? [];
  });

  ipcMain.handle('mg:trash-file', async (_event, filePath: string) => {
    await shell.trashItem(filePath);
    return true;
  });

  ipcMain.handle('mg:open-external', async (_event, url: string) => {
    if (url.startsWith('https://')) {
      await shell.openExternal(url);
    }
  });

  ipcMain.handle('mg:yt-download', async (_event, payload: YoutubeDownloadRequest) => {
    return youtube.enqueueAndDownload(payload, {
      deviceName: selfDevice.name,
      cacheDir: thumbCacheDir,
    });
  });

  ipcMain.handle('mg:pairing-session', () => {
    return pairing.createPairingSession({
      selfDevice: { ...selfDevice, transports: [...selfDevice.transports] },
      host: '127.0.0.1',
      port: 9443,
    });
  });
}

void app.whenReady().then(async () => {
  await bootstrapDesktopServices();
  createWindow();
});

app.on('window-all-closed', () => {
  watcher?.stop();
  void lanTransport?.stop();
  repo?.close();
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
