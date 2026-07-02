use crate::domain::{AccessCapability, DiskSource, TargetSafety};
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

pub fn classify_target_safety(
    source: DiskSource,
    is_internal: Option<bool>,
    is_removable: Option<bool>,
    protocol: Option<&str>,
) -> TargetSafety {
    if matches!(source, DiskSource::ImageFile) {
        return TargetSafety::Image;
    }
    if is_internal == Some(true) {
        return TargetSafety::Internal;
    }
    if is_removable == Some(true) {
        return TargetSafety::Removable;
    }
    if protocol
        .map(|value| value.eq_ignore_ascii_case("usb"))
        .unwrap_or(false)
        || is_internal == Some(false)
    {
        return TargetSafety::External;
    }
    TargetSafety::Unknown
}

pub fn access_capability(readable: bool, writable: bool) -> AccessCapability {
    match (readable, writable) {
        (true, true) => AccessCapability::ReadWrite,
        (true, false) => AccessCapability::ReadOnly,
        (false, _) => AccessCapability::RequiresElevation,
    }
}

/// Block device reader — supports disk images and raw devices.
pub struct DiskReader {
    file: File,
    pub path: String,
    pub size: u64,
}

impl DiskReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DiskError> {
        let requested_path = path.as_ref().to_string_lossy().to_string();
        let path_str = platform_device_path(&requested_path);
        let path_ref = Path::new(&path_str);

        let file = File::open(path_ref).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                DiskError::PermissionDenied(format!(
                    "无法读取 {path_str}，可能需要 root 权限 (sudo)"
                ))
            } else {
                DiskError::Io(e)
            }
        })?;

        let size = disk_size(&path_str)
            .or_else(|| disk_size(&requested_path))
            .unwrap_or_else(|| file.metadata().map(|meta| meta.len()).unwrap_or(0));
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

fn platform_device_path(path: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        return macos_raw_device_path(path);
    }
    #[cfg(not(target_os = "macos"))]
    {
        path.to_string()
    }
}

#[cfg(target_os = "macos")]
fn macos_raw_device_path(path: &str) -> String {
    if let Some(suffix) = path.strip_prefix("/dev/disk") {
        if !suffix.contains('s') && suffix.chars().all(|c| c.is_ascii_digit()) {
            return format!("/dev/rdisk{suffix}");
        }
    }
    path.to_string()
}

fn disk_size(path: &str) -> Option<u64> {
    #[cfg(target_os = "macos")]
    if let Some(size) = macos_disk_size(path) {
        return Some(size);
    }

    use std::io::{Seek, SeekFrom};
    let mut file = File::open(path).ok()?;
    if let Ok(meta) = file.metadata() {
        if meta.is_file() {
            return Some(meta.len());
        }
    }
    file.seek(SeekFrom::End(0)).ok()
}

#[cfg(target_os = "macos")]
fn macos_disk_size(path: &str) -> Option<u64> {
    use plist::Value;
    use std::process::Command;

    let output = Command::new("diskutil")
        .args(["info", "-plist", path])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = Value::from_reader_xml(output.stdout.as_slice()).ok()?;
    value
        .as_dictionary()
        .and_then(|dict| dict.get("TotalSize").or_else(|| dict.get("Size")))
        .and_then(Value::as_unsigned_integer)
}
