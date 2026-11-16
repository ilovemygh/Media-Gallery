declare module 'electron' {
  export const app: {
    getPath(name: string): string;
    whenReady(): Promise<void>;
    on(event: string, listener: () => void): void;
    quit(): void;
  };
  export class BrowserWindow {
    constructor(options: Record<string, unknown>);
    loadURL(url: string): Promise<void>;
    loadFile(filePath: string): Promise<void>;
  }
  export const ipcMain: {
    handle(channel: string, listener: (event: unknown, ...args: any[]) => unknown): void;
  };
  export const ipcRenderer: {
    invoke(channel: string, ...args: unknown[]): Promise<unknown>;
  };
  export const contextBridge: {
    exposeInMainWorld(apiKey: string, api: unknown): void;
  };
  export const shell: {
    trashItem(fullPath: string): Promise<void>;
    openExternal(url: string): Promise<void>;
  };
}
