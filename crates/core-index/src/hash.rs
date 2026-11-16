use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PARTIAL_WINDOW_BYTES: usize = 64 * 1024;
const STREAM_BUF_BYTES: usize = 256 * 1024;

/// Computes full-file BLAKE3 hex digest via buffered streaming I/O.
pub fn compute_file_blake3(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; STREAM_BUF_BYTES];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Computes BLAKE3 of an in-memory slice.
pub fn compute_bytes_blake3(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Computes a fast partial fingerprint (size + first/mid/last 64 KB BLAKE3 windows)
/// for duplicate detection across 100k+ files without full hashing overhead.
pub fn compute_partial_blake3(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    hasher.update(format!("size:{size}:").as_bytes());

    if size == 0 {
        return Ok(hasher.finalize().to_hex().to_string());
    }

    let size_usize = usize::try_from(size).unwrap_or(usize::MAX);
    if size_usize <= PARTIAL_WINDOW_BYTES * 3 {
        let mut buf = vec![0u8; size_usize];
        file.read_exact(&mut buf)?;
        hasher.update(&buf);
    } else {
        let mut window = vec![0u8; PARTIAL_WINDOW_BYTES];
        file.read_exact(&mut window)?;
        hasher.update(&window);

        let mid_offset = (size / 2).saturating_sub((PARTIAL_WINDOW_BYTES / 2) as u64);
        file.seek(SeekFrom::Start(mid_offset))?;
        file.read_exact(&mut window)?;
        hasher.update(&window);

        let end_offset = size.saturating_sub(PARTIAL_WINDOW_BYTES as u64);
        file.seek(SeekFrom::Start(end_offset))?;
        file.read_exact(&mut window)?;
        hasher.update(&window);
    }

    Ok(hasher.finalize().to_hex().to_string())
}
