use crate::domain::{AccessCapability, DiskInfo, DiskKind, DiskSource, TargetSafety};
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
            let display_path = trimmed
                .split_whitespace()
                .next()
                .unwrap_or(trimmed)
                .to_string();
            let path = platform_device_path(&display_path);
            let readable = File::open(&path).is_ok();
            disks.push(system_disk_info(
                display_path.clone(),
                path,
                disk_size(&display_path).unwrap_or(0),
                readable,
                None,
                None,
                None,
            ));
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
                && name
                    .chars()
                    .last()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                && !name.contains('p')
            {
                let path = format!("/dev/{name}");
                let readable = File::open(&path).is_ok();
                disks.push(system_disk_info(
                    path.clone(),
                    path.clone(),
                    disk_size(&path).unwrap_or(0),
                    readable,
                    None,
                    None,
                    None,
                ));
            }
        }
    }
    disks
}

fn fallback_disk_paths() -> Vec<DiskInfo> {
    (0..=3)
        .map(|i| format!("/dev/disk{i}"))
        .filter(|p| Path::new(p).exists())
        .map(|display_path| {
            let path = platform_device_path(&display_path);
            let readable = File::open(&path).is_ok();
            system_disk_info(
                display_path.clone(),
                path,
                disk_size(&display_path).unwrap_or(0),
                readable,
                None,
                None,
                None,
            )
        })
        .collect()
}

fn system_disk_info(
    display_path: String,
    raw_path: String,
    size_bytes: u64,
    readable: bool,
    protocol: Option<String>,
    is_internal: Option<bool>,
    is_removable: Option<bool>,
) -> DiskInfo {
    let platform_id = platform_id_from_path(&display_path);
    let safety = classify_target_safety(
        DiskSource::System,
        is_internal,
        is_removable,
        protocol.as_deref(),
    );

    DiskInfo {
        path: raw_path.clone(),
        display_path: display_path.clone(),
        raw_path,
        platform_id,
        name: display_path,
        size_bytes,
        readable,
        writable: false,
        source: DiskSource::System,
        kind: DiskKind::Physical,
        safety,
        access: access_capability(readable, false),
        protocol,
        is_internal,
        is_removable,
    }
}

fn platform_id_from_path(path: &str) -> String {
    path.strip_prefix("/dev/")
        .or_else(|| path.strip_prefix(r"\\.\"))
        .unwrap_or(path)
        .to_string()
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

pub fn disk_info_from_image(path: &str) -> Result<DiskInfo, DiskError> {
    let meta = std::fs::metadata(path)?;
    Ok(DiskInfo {
        path: path.to_string(),
        display_path: path.to_string(),
        raw_path: path.to_string(),
        platform_id: path.to_string(),
        name: Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string()),
        size_bytes: meta.len(),
        readable: true,
        writable: !meta.permissions().readonly(),
        source: DiskSource::ImageFile,
        kind: DiskKind::Image,
        safety: TargetSafety::Image,
        access: access_capability(true, !meta.permissions().readonly()),
        protocol: Some("file".to_string()),
        is_internal: Some(false),
        is_removable: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_image_targets_as_image_safety() {
        assert_eq!(
            classify_target_safety(DiskSource::ImageFile, None, None, None),
            TargetSafety::Image
        );
    }

    #[test]
    fn classifies_internal_system_disks_as_internal() {
        assert_eq!(
            classify_target_safety(
                DiskSource::System,
                Some(true),
                Some(false),
                Some("PCI-Express")
            ),
            TargetSafety::Internal
        );
    }

    #[test]
    fn classifies_usb_and_removable_disks() {
        assert_eq!(
            classify_target_safety(DiskSource::System, Some(false), Some(false), Some("USB")),
            TargetSafety::External
        );
        assert_eq!(
            classify_target_safety(DiskSource::System, None, Some(true), None),
            TargetSafety::Removable
        );
    }

    #[test]
    fn maps_access_flags_to_stable_capability() {
        assert_eq!(access_capability(true, true), AccessCapability::ReadWrite);
        assert_eq!(access_capability(true, false), AccessCapability::ReadOnly);
        assert_eq!(
            access_capability(false, false),
            AccessCapability::RequiresElevation
        );
    }

    #[test]
    fn builds_system_disk_info_with_normalized_fields() {
        let info = system_disk_info(
            "/dev/disk4".to_string(),
            "/dev/rdisk4".to_string(),
            1024,
            true,
            Some("USB".to_string()),
            Some(false),
            Some(true),
        );

        assert_eq!(info.platform_id, "disk4");
        assert_eq!(info.display_path, "/dev/disk4");
        assert_eq!(info.raw_path, "/dev/rdisk4");
        assert_eq!(info.safety, TargetSafety::Removable);
        assert_eq!(info.access, AccessCapability::ReadOnly);
    }
}
