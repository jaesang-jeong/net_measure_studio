use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileInfo {
    pub filename: String,
    pub size_bytes: u64,
    pub modified_time: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTransferStatus {
    Queued,
    Downloading,
    Verifying,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferProgress {
    pub filename: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub status: FileTransferStatus,
    pub hash_verified: bool,
}

pub struct FileTransferManager {
    is_busy: Arc<AtomicBool>,
}

impl FileTransferManager {
    pub fn new() -> Self {
        Self {
            is_busy: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::Relaxed)
    }

    /// Calculate SHA256 of a local file
    pub fn calculate_file_sha256(path: &Path) -> std::io::Result<String> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    /// Isolated background download worker to prevent any jitter on measurement loop
    pub fn download_file_background<F>(
        &self,
        file_info: RemoteFileInfo,
        dest_dir: PathBuf,
        delete_remote_after: bool,
        mut progress_callback: F,
    ) where
        F: FnMut(FileTransferProgress) + Send + 'static,
    {
        let is_busy = Arc::clone(&self.is_busy);
        is_busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            let filename = file_info.filename.clone();
            let total = file_info.size_bytes;

            progress_callback(FileTransferProgress {
                filename: filename.clone(),
                total_bytes: total,
                transferred_bytes: 0,
                status: FileTransferStatus::Downloading,
                hash_verified: false,
            });

            let dest_path = dest_dir.join(&filename);
            let result: Result<(), String> = (|| {
                let mut dest_file = File::create(&dest_path)
                    .map_err(|e| format!("Create destination file error: {}", e))?;

                // Simulate chunked high-speed transfer with progress
                let chunk_size = 64 * 1024; // 64 KB
                let mut written = 0u64;

                while written < total {
                    let step = (total - written).min(chunk_size as u64);
                    // Write dummy binary test stream or actual network payload
                    let chunk = vec![0xA5u8; step as usize];
                    dest_file.write_all(&chunk).map_err(|e| e.to_string())?;
                    written += step;

                    progress_callback(FileTransferProgress {
                        filename: filename.clone(),
                        total_bytes: total,
                        transferred_bytes: written,
                        status: FileTransferStatus::Downloading,
                        hash_verified: false,
                    });

                    // Small yield to simulate network rate without blocking CPU
                    thread::sleep(Duration::from_millis(20));
                }
                dest_file.flush().map_err(|e| e.to_string())?;

                // Hash verification
                progress_callback(FileTransferProgress {
                    filename: filename.clone(),
                    total_bytes: total,
                    transferred_bytes: total,
                    status: FileTransferStatus::Verifying,
                    hash_verified: false,
                });

                let computed_hash = Self::calculate_file_sha256(&dest_path)
                    .map_err(|e| format!("Hash calc failed: {}", e))?;

                let verified = if file_info.sha256.is_empty() {
                    true
                } else {
                    computed_hash.eq_ignore_ascii_case(&file_info.sha256)
                };

                if !verified && !file_info.sha256.is_empty() {
                    return Err(format!(
                        "Hash mismatch! Expected: {}, Computed: {}",
                        file_info.sha256, computed_hash
                    ));
                }

                if delete_remote_after {
                    // Option to delete file on target storage after successful recovery
                }

                progress_callback(FileTransferProgress {
                    filename: filename.clone(),
                    total_bytes: total,
                    transferred_bytes: total,
                    status: FileTransferStatus::Completed,
                    hash_verified: verified,
                });

                Ok(())
            })();

            if let Err(err_msg) = result {
                progress_callback(FileTransferProgress {
                    filename: filename.clone(),
                    total_bytes: total,
                    transferred_bytes: 0,
                    status: FileTransferStatus::Failed(err_msg),
                    hash_verified: false,
                });
            }

            is_busy.store(false, Ordering::SeqCst);
        });
    }
}
