//! SQLite + FTS5 media index, parallel BLAKE3 scanner, duplicate detector,
//! Android MediaStore ingestion, and read-only v0.1 legacy importer.

pub mod hash;
pub mod legacy_importer;
pub mod migrations;
pub mod models;
pub mod repository;
pub mod scanner;

pub use hash::{compute_bytes_blake3, compute_file_blake3, compute_partial_blake3};
pub use legacy_importer::import_legacy_sqlite;
pub use models::*;
pub use repository::{build_fts_match_query, now_ms, IndexError, MediaIndex};
pub use scanner::{
    classify_extension, ingest_android_mediastore_batch, scan_folders, stable_media_id,
    ProbedMediaMetadata, ScanRequest,
};
