use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::Command;

pub use testdisk_core::DiskInfo;
use testdisk_core::{AccessCapability, DiskError, DiskKind, DiskSource, TargetSafety};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetSource {
    System,
    ImageFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Physical,
    Virtual,
    Image,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetMountState {
    Mounted,
    Unmounted,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedTarget {
    pub normalized_id: String,
    pub display_path: String,
    pub raw_path: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: TargetKind,
    pub protocol: Option<String>,
    pub is_internal: Option<bool>,
    pub is_removable: Option<bool>,
    pub is_external: Option<bool>,
    pub mount_state: TargetMountState,
    pub readable: bool,
    pub writable: bool,
    pub permission_message: Option<String>,
    pub source: TargetSource,
}

pub fn classify_target_safety(
    source: TargetSource,
    is_internal: Option<bool>,
    is_removable: Option<bool>,
    protocol: Option<&str>,
) -> TargetSafety {
    if matches!(source, TargetSource::ImageFile) {
        return TargetSafety::Image;
    }
    if is_internal == Some(true) {
        return TargetSafety::Internal;
    }
    if is_removable == Some(true) {
        return TargetSafety::Removable;
    }
    if is_internal == Some(false)
        || protocol
            .map(|value| value.eq_ignore_ascii_case("usb"))
            .unwrap_or(false)
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

impl From<NormalizedTarget> for DiskInfo {
    fn from(target: NormalizedTarget) -> Self {
        let source = match target.source {
            TargetSource::System => DiskSource::System,
            TargetSource::ImageFile => DiskSource::ImageFile,
        };
        let kind = match target.kind {
            TargetKind::Image => DiskKind::Image,
            _ => DiskKind::Physical,
        };
        let safety = classify_target_safety(
            target.source.clone(),
            target.is_internal,
            target.is_removable,
            target.protocol.as_deref(),
        );

        DiskInfo {
            path: target.raw_path.clone(),
            display_path: target.display_path,
            raw_path: target.raw_path,
            platform_id: target.normalized_id,
            name: target.name,
            size_bytes: target.size_bytes,
            readable: target.readable,
            writable: target.writable,
            source,
            kind,
            safety,
            access: access_capability(target.readable, target.writable),
            protocol: target.protocol,
            is_internal: target.is_internal,
            is_removable: target.is_removable,
        }
    }
}

pub fn enumerate_targets() -> Vec<NormalizedTarget> {
    #[cfg(target_os = "macos")]
    {
        return enumerate_targets_macos();
    }
    #[cfg(target_os = "linux")]
    {
        return enumerate_targets_linux();
    }
    #[cfg(target_os = "windows")]
    {
        return enumerate_targets_windows();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        vec![]
    }
}

pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_targets()
        .into_iter()
        .map(DiskInfo::from)
        .collect()
}

pub fn disk_info_from_image(path: &str) -> Result<DiskInfo, DiskError> {
    Ok(DiskInfo::from(normalized_target_from_image(path)?))
}

pub fn normalized_target_from_image(path: &str) -> Result<NormalizedTarget, DiskError> {
    let meta = fs::metadata(path)?;
    let name = Path::new(path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    Ok(NormalizedTarget {
        normalized_id: format!("image:{path}"),
        display_path: path.to_string(),
        raw_path: path.to_string(),
        name,
        size_bytes: meta.len(),
        kind: TargetKind::Image,
        protocol: Some("file".to_string()),
        is_internal: Some(false),
        is_removable: None,
        is_external: Some(false),
        mount_state: TargetMountState::Unmounted,
        readable: true,
        writable: !meta.permissions().readonly(),
        permission_message: None,
        source: TargetSource::ImageFile,
    })
}

#[cfg(target_os = "macos")]
fn enumerate_targets_macos() -> Vec<NormalizedTarget> {
    let output = Command::new("diskutil")
        .args(["list", "-plist"])
        .output()
        .ok();
    let Some(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }

    use plist::Value;
    let value = Value::from_reader_xml(output.stdout.as_slice()).ok();
    let Some(value) = value else {
        return vec![];
    };
    let mut targets = Vec::new();
    let Some(devices) = value
        .as_dictionary()
        .and_then(|dict| dict.get("AllDisksAndPartitions"))
        .and_then(Value::as_array)
    else {
        return vec![];
    };
    for item in devices {
        let Some(dict) = item.as_dictionary() else {
            continue;
        };
        let Some(device_id) = plist_string(dict, "DeviceIdentifier") else {
            continue;
        };
        let device_node =
            plist_string(dict, "DeviceNode").unwrap_or_else(|| format!("/dev/{device_id}"));
        let raw_path = platform_device_path(&device_node);
        let size_bytes = plist_u64(dict, "Size").unwrap_or(0);
        let protocol = plist_string(dict, "BusProtocol");
        let is_internal = plist_bool(dict, "Internal");
        let is_removable = plist_bool(dict, "RemovableMedia");
        let mounted = plist_bool(dict, "Mounted").unwrap_or(false);
        let name = plist_string(dict, "VolumeName")
            .or_else(|| plist_string(dict, "MediaName"))
            .unwrap_or_else(|| device_id.clone());
        let (readable, permission_message) = probe_read_access(&raw_path);

        targets.push(NormalizedTarget {
            normalized_id: normalized_id_from_path(&device_node),
            display_path: device_node,
            raw_path,
            name,
            size_bytes,
            kind: TargetKind::Physical,
            protocol: protocol.map(|value| value.to_ascii_lowercase()),
            is_internal,
            is_removable,
            is_external: is_internal.map(|value| !value),
            mount_state: if mounted {
                TargetMountState::Mounted
            } else {
                TargetMountState::Unmounted
            },
            readable,
            writable: !plist_bool(dict, "ReadOnlyMedia").unwrap_or(false),
            permission_message,
            source: TargetSource::System,
        });
    }
    targets
}

#[cfg(target_os = "linux")]
fn enumerate_targets_linux() -> Vec<NormalizedTarget> {
    let mut command = Command::new("lsblk");
    command.args([
        "--json",
        "--bytes",
        "--paths",
        "--output",
        "NAME,PATH,TYPE,SIZE,RM,RO,MOUNTPOINT,MOUNTPOINTS,TRAN,MODEL,SERIAL,HOTPLUG",
    ]);

    if let Some(output) = run_command_output(&mut command) {
        if let Ok(targets) = parse_lsblk_targets(&output) {
            if !targets.is_empty() {
                return targets;
            }
        }
    }

    fallback_linux_targets()
}

#[cfg(target_os = "windows")]
fn enumerate_targets_windows() -> Vec<NormalizedTarget> {
    let mut command = Command::new("powershell");
    command.args([
        "-NoProfile",
        "-Command",
        "Get-Disk | Select-Object Number,FriendlyName,BusType,MediaType,Size,IsOffline,IsReadOnly,IsRemovable,IsVirtual | ConvertTo-Json -Depth 3",
    ]);

    if let Some(output) = run_command_output(&mut command) {
        if let Ok(targets) = parse_windows_targets(&output) {
            if !targets.is_empty() {
                return targets;
            }
        }
    }

    vec![]
}

#[cfg(target_os = "linux")]
#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Deserialize)]
struct LsblkDevice {
    name: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(rename = "type", default)]
    device_type: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    rm: Option<bool>,
    #[serde(default)]
    ro: Option<bool>,
    #[serde(default)]
    mountpoint: Option<String>,
    #[serde(default)]
    mountpoints: Option<Vec<Option<String>>>,
    #[serde(default)]
    tran: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    serial: Option<String>,
    #[serde(default)]
    hotplug: Option<bool>,
}

#[cfg(target_os = "linux")]
fn parse_lsblk_targets(json: &str) -> Result<Vec<NormalizedTarget>, serde_json::Error> {
    let parsed: LsblkOutput = serde_json::from_str(json)?;
    Ok(parsed
        .blockdevices
        .into_iter()
        .filter_map(lsblk_device_to_target)
        .collect())
}

#[cfg(target_os = "linux")]
fn lsblk_device_to_target(device: LsblkDevice) -> Option<NormalizedTarget> {
    let device_type = device.device_type.as_deref().unwrap_or("disk");
    if !matches!(device_type, "disk" | "loop" | "rom") {
        return None;
    }

    let display_path = device
        .path
        .clone()
        .unwrap_or_else(|| format!("/dev/{}", device.name));
    let raw_path = display_path.clone();
    let size_bytes = device
        .size
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mount_state =
        if mountpoints_present(device.mountpoint.as_ref(), device.mountpoints.as_ref()) {
            TargetMountState::Mounted
        } else {
            TargetMountState::Unmounted
        };
    let protocol = device.tran.clone().or_else(|| {
        if device.name.starts_with("nvme") {
            Some("nvme".to_string())
        } else if device.name.starts_with("sd") {
            Some("sata".to_string())
        } else if device.name.starts_with("vd") {
            Some("virtio".to_string())
        } else {
            None
        }
    });
    let removable = device
        .rm
        .or(device.hotplug)
        .or_else(|| protocol.as_deref().map(is_external_protocol));
    let (readable, permission_message) = probe_read_access(&raw_path);

    Some(NormalizedTarget {
        normalized_id: normalized_id_from_path(&raw_path),
        display_path,
        raw_path,
        name: device
            .model
            .or(device.serial)
            .unwrap_or_else(|| device.name.clone()),
        size_bytes,
        kind: if device_type == "loop" {
            TargetKind::Virtual
        } else {
            TargetKind::Physical
        },
        protocol: protocol.map(|value| value.to_ascii_lowercase()),
        is_internal: removable.map(|value| !value),
        is_removable: removable,
        is_external: removable,
        mount_state,
        readable,
        writable: device.ro.map(|value| !value).unwrap_or(readable),
        permission_message,
        source: TargetSource::System,
    })
}

#[cfg(target_os = "linux")]
fn mountpoints_present(
    mountpoint: Option<&String>,
    mountpoints: Option<&Vec<Option<String>>>,
) -> bool {
    if mountpoint.is_some() {
        return true;
    }

    mountpoints
        .into_iter()
        .flat_map(|entries| entries.iter())
        .any(|entry| {
            entry
                .as_deref()
                .map(|value| !value.is_empty())
                .unwrap_or(false)
        })
}

#[cfg(target_os = "linux")]
fn is_external_protocol(protocol: &str) -> bool {
    matches!(
        protocol.to_ascii_lowercase().as_str(),
        "usb" | "sata" | "sas" | "nvme" | "sdio" | "virtio" | "thunderbolt"
    )
}

#[cfg(target_os = "linux")]
fn fallback_linux_targets() -> Vec<NormalizedTarget> {
    let mut targets = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let sys_path = entry.path();
            let raw_path = format!("/dev/{name}");
            let size_bytes = read_u64(sys_path.join("size")).unwrap_or(0) * 512;
            let removable = read_bool(sys_path.join("removable"));
            let (readable, permission_message) = probe_read_access(&raw_path);

            targets.push(NormalizedTarget {
                normalized_id: normalized_id_from_path(&raw_path),
                display_path: raw_path.clone(),
                raw_path,
                name: name.clone(),
                size_bytes,
                kind: if name.starts_with("loop") {
                    TargetKind::Virtual
                } else {
                    TargetKind::Physical
                },
                protocol: if name.starts_with("nvme") {
                    Some("nvme".to_string())
                } else if name.starts_with("sd") {
                    Some("sata".to_string())
                } else if name.starts_with("vd") {
                    Some("virtio".to_string())
                } else {
                    None
                },
                is_internal: removable.map(|value| !value),
                is_removable: removable,
                is_external: removable,
                mount_state: TargetMountState::Unknown,
                readable,
                writable: read_bool(sys_path.join("ro"))
                    .map(|value| !value)
                    .unwrap_or(false),
                permission_message,
                source: TargetSource::System,
            });
        }
    }
    targets
}

#[cfg(target_os = "windows")]
fn parse_windows_targets(json: &str) -> Result<Vec<NormalizedTarget>, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let entries = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => vec![],
    };

    Ok(entries
        .into_iter()
        .filter_map(|entry| entry.as_object().cloned())
        .map(|object| {
            let number = object
                .get("Number")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let raw_path = format!(r"\\.\PhysicalDrive{number}");
            let bus_type = object
                .get("BusType")
                .and_then(|value| value.as_str())
                .map(|value| value.to_ascii_lowercase());
            let is_offline = object
                .get("IsOffline")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let is_read_only = object
                .get("IsReadOnly")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let is_removable = object
                .get("IsRemovable")
                .and_then(|value| value.as_bool())
                .or_else(|| bus_type.as_deref().map(is_removable_bus_type));

            NormalizedTarget {
                normalized_id: format!("physicaldrive{number}"),
                display_path: raw_path.clone(),
                raw_path,
                name: object
                    .get("FriendlyName")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| format!("PhysicalDrive{number}")),
                size_bytes: object
                    .get("Size")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0),
                kind: if object
                    .get("IsVirtual")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
                {
                    TargetKind::Virtual
                } else {
                    TargetKind::Physical
                },
                protocol: bus_type,
                is_internal: is_removable.map(|value| !value),
                is_removable,
                is_external: is_removable,
                mount_state: if is_offline {
                    TargetMountState::Unmounted
                } else {
                    TargetMountState::Mounted
                },
                readable: !is_offline,
                writable: !is_offline && !is_read_only,
                permission_message: if is_offline {
                    Some("disk is offline or unavailable".to_string())
                } else {
                    None
                },
                source: TargetSource::System,
            }
        })
        .collect())
}

#[cfg(target_os = "windows")]
fn is_removable_bus_type(value: &str) -> bool {
    matches!(value, "usb" | "sd" | "external")
}

#[cfg(target_os = "macos")]
fn normalized_id_from_path(path: &str) -> String {
    path.strip_prefix("/dev/")
        .or_else(|| path.strip_prefix("/private/dev/"))
        .unwrap_or(path)
        .trim_start_matches('r')
        .to_ascii_lowercase()
}

#[cfg(not(target_os = "macos"))]
fn normalized_id_from_path(path: &str) -> String {
    let cleaned = path.strip_prefix("/dev/").unwrap_or(path);
    let cleaned = cleaned.strip_prefix("\\\\.\\").unwrap_or(cleaned);
    let cleaned = cleaned.strip_prefix("\\\\?\\").unwrap_or(cleaned);
    cleaned.to_ascii_lowercase()
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

fn probe_read_access(path: &str) -> (bool, Option<String>) {
    match File::open(path) {
        Ok(_) => (true, None),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => (
            false,
            Some(format!("无法读取 {path}，可能需要 root 权限 (sudo)")),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            (false, Some(format!("目标不存在: {path}")))
        }
        Err(_) => (true, None),
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn run_command_output(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "linux")]
fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok()
}

#[cfg(target_os = "linux")]
fn read_bool(path: impl AsRef<Path>) -> Option<bool> {
    read_to_string(path).and_then(|value| match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    })
}

#[cfg(target_os = "linux")]
fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_to_string(path).and_then(|value| value.trim().parse::<u64>().ok())
}

#[cfg(target_os = "macos")]
fn plist_string(dict: &plist::Dictionary, key: &str) -> Option<String> {
    dict.get(key)
        .and_then(plist::Value::as_string)
        .map(|value| value.to_string())
}

#[cfg(target_os = "macos")]
fn plist_bool(dict: &plist::Dictionary, key: &str) -> Option<bool> {
    dict.get(key).and_then(plist::Value::as_boolean)
}

#[cfg(target_os = "macos")]
fn plist_u64(dict: &plist::Dictionary, key: &str) -> Option<u64> {
    dict.get(key).and_then(plist::Value::as_unsigned_integer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalizes_system_paths_stably() {
        assert_eq!(normalized_id_from_path("/dev/disk4"), "disk4");
        #[cfg(target_os = "windows")]
        assert_eq!(
            normalized_id_from_path(r"\\.\PhysicalDrive12"),
            "physicaldrive12"
        );
        #[cfg(not(target_os = "windows"))]
        assert_eq!(
            normalized_id_from_path(r"\\.\PhysicalDrive12"),
            r"\\.\physicaldrive12"
        );
    }

    #[test]
    fn maps_image_targets_with_stable_metadata() {
        let temp_path: PathBuf = std::env::temp_dir().join("testdisk-platform-image.img");
        std::fs::write(&temp_path, vec![0u8; 1024]).unwrap();
        let target = normalized_target_from_image(temp_path.to_str().unwrap()).unwrap();
        assert_eq!(target.source, TargetSource::ImageFile);
        assert_eq!(target.kind, TargetKind::Image);
        assert_eq!(target.protocol.as_deref(), Some("file"));
        assert_eq!(target.is_internal, Some(false));
        assert_eq!(target.is_external, Some(false));
        assert_eq!(target.mount_state, TargetMountState::Unmounted);
        let _ = std::fs::remove_file(temp_path);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_lsblk_json_into_normalized_targets() {
        let json = r#"
        {
          "blockdevices": [
            {
              "name": "sda",
              "path": "/dev/sda",
              "type": "disk",
              "size": "1000204886016",
              "rm": false,
              "ro": false,
              "mountpoint": null,
              "mountpoints": [null],
              "tran": "sata",
              "model": "Sample SSD",
              "serial": "ABC123",
              "hotplug": false
            }
          ]
        }
        "#;

        let targets = parse_lsblk_targets(json).unwrap();
        assert_eq!(targets.len(), 1);
        let target = &targets[0];
        assert_eq!(target.normalized_id, "sda");
        assert_eq!(target.display_path, "/dev/sda");
        assert_eq!(target.protocol.as_deref(), Some("sata"));
        assert_eq!(target.kind, TargetKind::Physical);
        assert_eq!(target.mount_state, TargetMountState::Unmounted);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_windows_disk_json_into_normalized_targets() {
        let json = r#"
        [
          {
            "Number": 2,
            "FriendlyName": "USB Drive",
            "BusType": "USB",
            "MediaType": "Removable Media",
            "Size": 64023257088,
            "IsOffline": false,
            "IsReadOnly": false,
            "IsRemovable": true
          }
        ]
        "#;

        let targets = parse_windows_targets(json).unwrap();
        assert_eq!(targets.len(), 1);
        let target = &targets[0];
        assert_eq!(target.normalized_id, "physicaldrive2");
        assert_eq!(target.protocol.as_deref(), Some("usb"));
        assert_eq!(target.is_removable, Some(true));
        assert_eq!(target.mount_state, TargetMountState::Mounted);
    }
}
