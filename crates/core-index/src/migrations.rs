use rusqlite::Connection;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS device (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('phone', 'pc')),
  public_key TEXT NOT NULL DEFAULT '',
  last_seen INTEGER NOT NULL,
  transports TEXT NOT NULL DEFAULT '[]',
  host TEXT,
  port INTEGER,
  is_self INTEGER NOT NULL DEFAULT 0,
  is_online INTEGER NOT NULL DEFAULT 0,
  paired_at INTEGER
);

CREATE TABLE IF NOT EXISTS media (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  path TEXT NOT NULL,
  folder_path TEXT NOT NULL,
  file_name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('video', 'image', 'audio')),
  mime TEXT NOT NULL,
  size INTEGER NOT NULL,
  blake3 TEXT NOT NULL DEFAULT '',
  partial_hash TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  modified_at INTEGER NOT NULL,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  width INTEGER NOT NULL DEFAULT 0,
  height INTEGER NOT NULL DEFAULT 0,
  gps_lat REAL,
  gps_lon REAL,
  place TEXT,
  tags TEXT NOT NULL DEFAULT '{}',
  alias TEXT NOT NULL,
  auto_alias TEXT NOT NULL,
  cover_id TEXT,
  sync_state TEXT NOT NULL CHECK(sync_state IN ('Local', 'Remote', 'Downloading', 'Synced', 'Error')),
  is_duplicate INTEGER NOT NULL DEFAULT 0,
  duplicate_group_id TEXT,
  is_trashed INTEGER NOT NULL DEFAULT 0,
  trashed_path TEXT,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_media_device_path ON media(device_id, path);
CREATE INDEX IF NOT EXISTS idx_media_type ON media(type);
CREATE INDEX IF NOT EXISTS idx_media_folder ON media(folder_path);
CREATE INDEX IF NOT EXISTS idx_media_created ON media(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_size_partial ON media(size, partial_hash);
CREATE INDEX IF NOT EXISTS idx_media_sync ON media(sync_state);
CREATE INDEX IF NOT EXISTS idx_media_trashed ON media(is_trashed);

CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(
  media_id UNINDEXED,
  file_name,
  alias,
  auto_alias,
  place,
  tags,
  folder_path,
  device_id,
  date_text,
  tokenize = 'unicode61 remove_diacritics 1'
);

CREATE TRIGGER IF NOT EXISTS media_ai AFTER INSERT ON media BEGIN
  INSERT INTO media_fts(media_id, file_name, alias, auto_alias, place, tags, folder_path, device_id, date_text)
  VALUES (
    new.id, new.file_name, new.alias, new.auto_alias, COALESCE(new.place, ''),
    new.tags, new.folder_path, new.device_id,
    strftime('%Y-%m-%d', new.created_at / 1000, 'unixepoch')
  );
END;

CREATE TRIGGER IF NOT EXISTS media_au AFTER UPDATE ON media BEGIN
  DELETE FROM media_fts WHERE media_id = old.id;
  INSERT INTO media_fts(media_id, file_name, alias, auto_alias, place, tags, folder_path, device_id, date_text)
  VALUES (
    new.id, new.file_name, new.alias, new.auto_alias, COALESCE(new.place, ''),
    new.tags, new.folder_path, new.device_id,
    strftime('%Y-%m-%d', new.created_at / 1000, 'unixepoch')
  );
END;

CREATE TRIGGER IF NOT EXISTS media_ad AFTER DELETE ON media BEGIN
  DELETE FROM media_fts WHERE media_id = old.id;
END;

CREATE TABLE IF NOT EXISTS cover (
  id TEXT PRIMARY KEY,
  media_id TEXT NOT NULL UNIQUE,
  source TEXT NOT NULL CHECK(source IN ('generated', 'custom', 'embedded')),
  file_path TEXT NOT NULL,
  cloud_id TEXT,
  aspect TEXT NOT NULL DEFAULT '16:9',
  crop TEXT,
  frame_time_ms INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS transfer (
  id TEXT PRIMARY KEY,
  media_id TEXT NOT NULL,
  file_name TEXT NOT NULL,
  from_device TEXT NOT NULL,
  to_device TEXT NOT NULL,
  status TEXT NOT NULL,
  bytes_done INTEGER NOT NULL DEFAULT 0,
  bytes_total INTEGER NOT NULL DEFAULT 0,
  last_chunk_index INTEGER NOT NULL DEFAULT 0,
  chunk_size INTEGER NOT NULL DEFAULT 262144,
  path_kind TEXT NOT NULL,
  speed_bps INTEGER NOT NULL DEFAULT 0,
  expected_blake3 TEXT NOT NULL DEFAULT '',
  error TEXT,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS pinned_folder (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  path TEXT NOT NULL,
  label TEXT NOT NULL,
  icon TEXT NOT NULL DEFAULT 'folder',
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS rename_log (
  id TEXT PRIMARY KEY,
  media_id TEXT NOT NULL,
  batch_id TEXT,
  old_name TEXT NOT NULL,
  new_name TEXT NOT NULL,
  old_path TEXT NOT NULL,
  new_path TEXT NOT NULL,
  applied_at INTEGER NOT NULL,
  undone_at INTEGER
);

CREATE TABLE IF NOT EXISTS download_job (
  id TEXT PRIMARY KEY,
  url TEXT NOT NULL,
  title TEXT,
  format TEXT NOT NULL CHECK(format IN ('mp4', 'mp3')),
  quality TEXT NOT NULL,
  destination_device_id TEXT NOT NULL,
  destination_folder TEXT NOT NULL,
  status TEXT NOT NULL,
  progress REAL NOT NULL DEFAULT 0,
  output_media_id TEXT,
  error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS saved_search (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  query TEXT NOT NULL,
  filters TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS auto_sync_rule (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  source_device_id TEXT NOT NULL,
  target_device_id TEXT NOT NULL,
  folder_pattern TEXT NOT NULL,
  media_types TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL
);
"#;

const MIGRATION_V2: &str = r#"
CREATE INDEX IF NOT EXISTS idx_media_blake3 ON media(blake3);
CREATE INDEX IF NOT EXISTS idx_rename_log_media ON rename_log(media_id, applied_at DESC);
"#;

/// Applies forward-only schema migrations inside transactions.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS schema_migrations (
           version INTEGER PRIMARY KEY,
           applied_at INTEGER NOT NULL
         );",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let migrations: &[(i64, &str)] = &[(1, MIGRATION_V1), (2, MIGRATION_V2)];
    for (version, sql) in migrations {
        if current_version < *version {
            conn.execute_batch("BEGIN IMMEDIATE;")?;
            if let Err(err) = conn.execute_batch(sql) {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now') * 1000)",
                [version],
            )?;
            conn.execute_batch("COMMIT;")?;
        }
    }
    Ok(())
}
