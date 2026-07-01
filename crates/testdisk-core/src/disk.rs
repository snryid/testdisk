use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

pub const SECTOR_SIZE: u64 = 512;

#[derive(Debug, Error)]
pub enum DiskError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Disk not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub readable: bool,
    pub source: DiskSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskSource {
    System,
    ImageFile,
}

/// Block device reader — supports disk images and raw devices.
pub struct DiskReader {
    file: File,
    pub path: String,
    pub size: u64,
}

impl DiskReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DiskError> {
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();

        let file = File::open(path_ref).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                DiskError::PermissionDenied(format!(
                    "无法读取 {path_str}，可能需要 root 权限 (sudo)"
                ))
            } else {
                DiskError::Io(e)
            }
        })?;

        let size = file.metadata()?.len();
        Ok(Self {
            file,
            path: path_str,
            size,
        })
    }

    pub fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, DiskError> {
        self.file.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; len];
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_sector(&mut self, lba: u64) -> Result<Vec<u8>, DiskError> {
        self.read_at(lba * SECTOR_SIZE, SECTOR_SIZE as usize)
    }
}

/// Enumerate available disks (macOS via diskutil, fallback to common paths).
pub fn list_disks() -> Vec<DiskInfo> {
    #[cfg(target_os = "macos")]
    {
        list_disks_macos()
    }
    #[cfg(target_os = "linux")]
    {
        list_disks_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

#[cfg(target_os = "macos")]
fn list_disks_macos() -> Vec<DiskInfo> {
    use std::process::Command;

    let output = Command::new("diskutil").arg("list").output();
    let Ok(out) = output else {
        return fallback_disk_paths();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut disks = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/dev/disk") && !trimmed.contains('s') {
            let path = trimmed.split_whitespace().next().unwrap_or(trimmed).to_string();
            let readable = File::open(&path).is_ok();
            disks.push(DiskInfo {
                name: path.clone(),
                path: path.clone(),
                size_bytes: disk_size(&path).unwrap_or(0),
                readable,
                source: DiskSource::System,
            });
        }
    }

    if disks.is_empty() {
        fallback_disk_paths()
    } else {
        disks
    }
}

#[cfg(target_os = "linux")]
fn list_disks_linux() -> Vec<DiskInfo> {
    let mut disks = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (name.starts_with("sd") || name.starts_with("nvme") || name.starts_with("vd"))
                && name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && !name.contains('p')
            {
                let path = format!("/dev/{name}");
                let readable = File::open(&path).is_ok();
                disks.push(DiskInfo {
                    name: name.clone(),
                    path: path.clone(),
                    size_bytes: disk_size(&path).unwrap_or(0),
                    readable,
                    source: DiskSource::System,
                });
            }
        }
    }
    disks
}

fn fallback_disk_paths() -> Vec<DiskInfo> {
    (0..=3)
        .map(|i| format!("/dev/disk{i}"))
        .filter(|p| Path::new(p).exists())
        .map(|path| {
            let readable = File::open(&path).is_ok();
            DiskInfo {
                name: path.clone(),
                path: path.clone(),
                size_bytes: disk_size(&path).unwrap_or(0),
                readable,
                source: DiskSource::System,
            }
        })
        .collect()
}

fn disk_size(path: &str) -> Option<u64> {
    use std::io::{Seek, SeekFrom};
    let mut file = File::open(path).ok()?;
    if let Ok(meta) = file.metadata() {
        if meta.is_file() {
            return Some(meta.len());
        }
    }
    file.seek(SeekFrom::End(0)).ok()
}

pub fn disk_info_from_image(path: &str) -> Result<DiskInfo, DiskError> {
    let meta = std::fs::metadata(path)?;
    Ok(DiskInfo {
        path: path.to_string(),
        name: Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string()),
        size_bytes: meta.len(),
        readable: true,
        source: DiskSource::ImageFile,
    })
}
