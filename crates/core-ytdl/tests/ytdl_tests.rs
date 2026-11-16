use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    DeviceRecord, DeviceType, DownloadFormat, DownloadStatus, MediaFilter, MediaIndex, MediaType,
    SortSpec, TransportKind,
};
use core_platform::{DesktopFfmpegSidecar, PlatformError, YtDlpBackend};
use core_ytdl::{build_yt_dlp_args, YoutubeDownloadRequest, YoutubeDownloader};

struct MockYtBackend {
    out_file: PathBuf,
}

impl YtDlpBackend for MockYtBackend {
    fn run_download(
        &self,
        _args: &[String],
        on_progress: &(dyn Fn(f32, Option<String>) + Send + Sync),
    ) -> Result<Vec<PathBuf>, PlatformError> {
        on_progress(50.0, Some("Synthwave Mix".into()));
        std::fs::write(&self.out_file, b"ID3-FAKE-MP3-DATA")?;
        on_progress(100.0, Some("Synthwave Mix".into()));
        Ok(vec![self.out_file.clone()])
    }

    fn run_update(&self) -> Result<String, PlatformError> {
        Ok("yt-dlp is up to date".into())
    }
}

#[test]
fn builds_args_updates_ytdlp_and_auto_ingests_mp3_into_library() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mg-ytdl-{nanos}"));
    let dl_dir = dir.join("music");
    let cache_dir = dir.join("cache");
    std::fs::create_dir_all(&dl_dir).unwrap();

    let mp4_args = build_yt_dlp_args(&YoutubeDownloadRequest {
        url: "https://youtube.com/watch?v=123".into(),
        format: DownloadFormat::Mp4,
        quality: "1080p".into(),
        destination_device_id: "pc-1".into(),
        destination_folder: dl_dir.to_string_lossy().to_string(),
        allow_playlist: false,
        ffmpeg_bin: None,
    });
    assert!(mp4_args.contains(&"--no-playlist".to_string()));
    assert!(mp4_args.contains(&"--embed-thumbnail".to_string()));
    assert!(mp4_args.iter().any(|a| a.contains("height<=1080")));

    let idx = MediaIndex::open_in_memory().unwrap();
    idx.upsert_device(&DeviceRecord {
        id: "pc-1".into(),
        name: "PC".into(),
        device_type: DeviceType::Pc,
        public_key: "pk".into(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: None,
        port: None,
        is_self: true,
        is_online: true,
        paired_at: None,
    })
    .unwrap();

    let out_mp3 = dl_dir.join("Synthwave_Study_Mix.mp3");
    let downloader = YoutubeDownloader::new(
        idx.clone(),
        Arc::new(MockYtBackend { out_file: out_mp3 }),
        Arc::new(DesktopFfmpegSidecar::default()),
        &cache_dir,
    );
    assert!(downloader.update_yt_dlp().unwrap().contains("up to date"));

    let (job, imported) = downloader
        .enqueue_and_download(
            &YoutubeDownloadRequest {
                url: "https://youtube.com/watch?v=demo".into(),
                format: DownloadFormat::Mp3,
                quality: "192k".into(),
                destination_device_id: "pc-1".into(),
                destination_folder: dl_dir.to_string_lossy().to_string(),
                allow_playlist: false,
                ffmpeg_bin: None,
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(job.status, DownloadStatus::Completed);
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].media_type, MediaType::Audio);
    assert_eq!(imported[0].tags.source.as_deref(), Some("youtube"));

    let audio_page = idx
        .list_media(
            &MediaFilter {
                media_type: Some(MediaType::Audio),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            50,
        )
        .unwrap();
    assert_eq!(audio_page.total_count, 1);
    assert!(audio_page.items[0].cover.is_some());

    let _ = std::fs::remove_dir_all(dir);
}
