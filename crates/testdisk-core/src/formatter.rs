use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Unsupported platform: disk formatting is only implemented for macOS")]
    UnsupportedPlatform,
    #[error("Invalid disk target: {0}")]
    InvalidTarget(String),
    #[error("Refusing to format internal or unknown disk: {0}")]
    UnsafeTarget(String),
    #[error("Confirmation does not match target disk")]
    ConfirmationMismatch,
    #[error("Invalid volume name")]
    InvalidVolumeName,
    #[error("Format command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FormatFilesystem {
    Apfs,
    Exfat,
    Fat32,
    HfsPlus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatFilesystemOption {
    pub value: FormatFilesystem,
    pub label: &'static str,
    pub diskutil_format: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDiskRequest {
    pub path: String,
    pub filesystem: FormatFilesystem,
    pub volume_name: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDiskResult {
    pub disk_identifier: String,
    pub filesystem: FormatFilesystem,
    pub volume_name: String,
    pub message: String,
}

pub fn format_filesystem_options() -> Vec<FormatFilesystemOption> {
    vec![
        FormatFilesystemOption {
            value: FormatFilesystem::Apfs,
            label: "APFS",
            diskutil_format: "APFS",
        },
        FormatFilesystemOption {
            value: FormatFilesystem::Exfat,
            label: "ExFAT",
            diskutil_format: "ExFAT",
        },
        FormatFilesystemOption {
            value: FormatFilesystem::Fat32,
            label: "MS-DOS FAT32",
            diskutil_format: "MS-DOS",
        },
        FormatFilesystemOption {
            value: FormatFilesystem::HfsPlus,
            label: "Mac OS Extended (Journaled)",
            diskutil_format: "JHFS+",
        },
    ]
}

pub fn format_disk(request: FormatDiskRequest) -> Result<FormatDiskResult, FormatError> {
    #[cfg(target_os = "macos")]
    {
        format_disk_macos(request)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        Err(FormatError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "macos")]
fn format_disk_macos(request: FormatDiskRequest) -> Result<FormatDiskResult, FormatError> {
    let plan = build_macos_erase_disk_plan(&request)?;
    if !macos_disk_is_external_or_removable(&plan.disk_identifier) {
        return Err(FormatError::UnsafeTarget(plan.disk_identifier));
    }

    let output = Command::new("diskutil").args(&plan.args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(FormatError::CommandFailed(if stderr.is_empty() {
            stdout
        } else {
            stderr
        }));
    }

    Ok(FormatDiskResult {
        disk_identifier: plan.disk_identifier,
        filesystem: request.filesystem,
        volume_name: plan.volume_name,
        message: String::from_utf8_lossy(&output.stdout).trim().to_string(),
    })
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
struct MacosEraseDiskPlan {
    disk_identifier: String,
    volume_name: String,
    args: Vec<String>,
}

#[cfg(target_os = "macos")]
fn build_macos_erase_disk_plan(
    request: &FormatDiskRequest,
) -> Result<MacosEraseDiskPlan, FormatError> {
    let disk_identifier = normalize_whole_disk_identifier(&request.path)?;
    validate_confirmation(&request.confirmation, &disk_identifier)?;
    let volume_name = validate_volume_name(&request.volume_name)?;
    let format = diskutil_format(&request.filesystem);

    Ok(MacosEraseDiskPlan {
        disk_identifier: disk_identifier.clone(),
        volume_name: volume_name.clone(),
        args: vec![
            "eraseDisk".to_string(),
            format.to_string(),
            volume_name,
            disk_identifier,
        ],
    })
}

#[cfg(target_os = "macos")]
fn normalize_whole_disk_identifier(path: &str) -> Result<String, FormatError> {
    let trimmed = path.trim();
    let identifier = trimmed
        .strip_prefix("/dev/rdisk")
        .or_else(|| trimmed.strip_prefix("/dev/disk"))
        .or_else(|| trimmed.strip_prefix("disk"))
        .ok_or_else(|| FormatError::InvalidTarget(path.to_string()))?;

    if identifier.is_empty()
        || identifier.contains('s')
        || !identifier.chars().all(|c| c.is_ascii_digit())
    {
        return Err(FormatError::InvalidTarget(path.to_string()));
    }

    Ok(format!("disk{identifier}"))
}

#[cfg(target_os = "macos")]
fn validate_confirmation(confirmation: &str, disk_identifier: &str) -> Result<(), FormatError> {
    let candidates = [
        disk_identifier.to_string(),
        format!("/dev/{disk_identifier}"),
        disk_identifier.replacen("disk", "/dev/rdisk", 1),
    ];

    if candidates
        .iter()
        .any(|candidate| confirmation.trim() == candidate)
    {
        Ok(())
    } else {
        Err(FormatError::ConfirmationMismatch)
    }
}

#[cfg(target_os = "macos")]
fn validate_volume_name(name: &str) -> Result<String, FormatError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains(':') {
        return Err(FormatError::InvalidVolumeName);
    }
    Ok(trimmed.to_string())
}

#[cfg(target_os = "macos")]
fn diskutil_format(filesystem: &FormatFilesystem) -> &'static str {
    format_filesystem_options()
        .into_iter()
        .find(|option| option.value == *filesystem)
        .map(|option| option.diskutil_format)
        .unwrap_or("ExFAT")
}

#[cfg(target_os = "macos")]
fn macos_disk_is_external_or_removable(disk_identifier: &str) -> bool {
    let output = Command::new("diskutil")
        .args(["info", "-plist", disk_identifier])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    macos_info_plist_is_external_or_removable(&output.stdout)
}

#[cfg(target_os = "macos")]
fn macos_info_plist_is_external_or_removable(bytes: &[u8]) -> bool {
    let Ok(value) = plist::Value::from_reader_xml(bytes) else {
        return false;
    };
    let Some(dict) = value.as_dictionary() else {
        return false;
    };

    let internal = dict
        .get("Internal")
        .and_then(plist::Value::as_boolean)
        .unwrap_or(true);
    let removable = dict
        .get("Removable")
        .and_then(plist::Value::as_boolean)
        .unwrap_or(false);
    let ejectable = dict
        .get("Ejectable")
        .and_then(plist::Value::as_boolean)
        .unwrap_or(false);
    let protocol = dict
        .get("Protocol")
        .and_then(plist::Value::as_string)
        .unwrap_or("");

    !internal || removable || ejectable || protocol.eq_ignore_ascii_case("USB")
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    #[test]
    fn exposes_supported_mainstream_filesystems() {
        let options = format_filesystem_options();

        let labels: Vec<&str> = options.iter().map(|option| option.label).collect();

        assert_eq!(
            labels,
            vec![
                "APFS",
                "ExFAT",
                "MS-DOS FAT32",
                "Mac OS Extended (Journaled)"
            ]
        );
    }

    #[test]
    fn builds_diskutil_erase_disk_plan_for_whole_raw_disk() {
        let request = FormatDiskRequest {
            path: "/dev/rdisk4".to_string(),
            filesystem: FormatFilesystem::Exfat,
            volume_name: "USB".to_string(),
            confirmation: "/dev/rdisk4".to_string(),
        };

        let plan = build_macos_erase_disk_plan(&request).unwrap();

        assert_eq!(plan.disk_identifier, "disk4");
        assert_eq!(plan.args, vec!["eraseDisk", "ExFAT", "USB", "disk4"]);
    }

    #[test]
    fn rejects_partition_targets() {
        let request = FormatDiskRequest {
            path: "/dev/disk4s1".to_string(),
            filesystem: FormatFilesystem::Apfs,
            volume_name: "USB".to_string(),
            confirmation: "/dev/disk4s1".to_string(),
        };

        let error = build_macos_erase_disk_plan(&request).unwrap_err();

        assert!(matches!(error, FormatError::InvalidTarget(_)));
    }

    #[test]
    fn rejects_confirmation_mismatch() {
        let request = FormatDiskRequest {
            path: "/dev/disk4".to_string(),
            filesystem: FormatFilesystem::Apfs,
            volume_name: "USB".to_string(),
            confirmation: "/dev/disk5".to_string(),
        };

        let error = build_macos_erase_disk_plan(&request).unwrap_err();

        assert!(matches!(error, FormatError::ConfirmationMismatch));
    }

    #[test]
    fn rejects_unsafe_volume_names() {
        let request = FormatDiskRequest {
            path: "/dev/disk4".to_string(),
            filesystem: FormatFilesystem::Apfs,
            volume_name: "Bad/Name".to_string(),
            confirmation: "/dev/disk4".to_string(),
        };

        let error = build_macos_erase_disk_plan(&request).unwrap_err();

        assert!(matches!(error, FormatError::InvalidVolumeName));
    }

    #[test]
    fn detects_usb_disk_from_diskutil_plist() {
        let plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Internal</key><false/>
  <key>Removable</key><true/>
  <key>Protocol</key><string>USB</string>
</dict>
</plist>"#;

        assert!(macos_info_plist_is_external_or_removable(plist));
    }

    #[test]
    fn rejects_internal_disk_from_diskutil_plist() {
        let plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Internal</key><true/>
  <key>Removable</key><false/>
  <key>Protocol</key><string>PCI-Express</string>
</dict>
</plist>"#;

        assert!(!macos_info_plist_is_external_or_removable(plist));
    }
}
