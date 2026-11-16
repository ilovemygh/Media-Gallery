import { contextBridge, ipcRenderer } from 'electron';

export interface ElectronMediaGalleryApi {
  scanFolder(folderPath: string): Promise<unknown>;
  queryMedia(query: string, filters: Record<string, unknown>): Promise<unknown>;
  trashFile(filePath: string): Promise<boolean>;
  openExternal(url: string): Promise<void>;
  startYoutubeDownload(payload: Record<string, unknown>): Promise<unknown>;
  getPairingSession(): Promise<unknown>;
}

const api: ElectronMediaGalleryApi = {
  scanFolder: (folderPath) => ipcRenderer.invoke('mg:scan-folder', folderPath),
  queryMedia: (query, filters) => ipcRenderer.invoke('mg:query-media', query, filters),
  trashFile: (filePath) => ipcRenderer.invoke('mg:trash-file', filePath) as Promise<boolean>,
  openExternal: (url) => ipcRenderer.invoke('mg:open-external', url) as Promise<void>,
  startYoutubeDownload: (payload) => ipcRenderer.invoke('mg:yt-download', payload),
  getPairingSession: () => ipcRenderer.invoke('mg:pairing-session'),
};

contextBridge.exposeInMainWorld('electronAPI', api);
