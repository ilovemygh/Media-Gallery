export interface CapacitorConfig {
  appId: string;
  appName: string;
  webDir: string;
  bundledWebRuntime: boolean;
  server: {
    androidScheme: string;
  };
  plugins: Record<string, Record<string, unknown>>;
}

const config: CapacitorConfig = {
  appId: 'io.mediagallery.app',
  appName: 'Media Gallery',
  webDir: 'dist',
  bundledWebRuntime: false,
  server: {
    androidScheme: 'https',
  },
  plugins: {
    MediaGalleryPlugin: {
      enableMediaStoreObserver: true,
      enableShareTarget: true,
      foregroundServiceChannelId: 'media_gallery_active_tasks',
    },
  },
};

export default config;
