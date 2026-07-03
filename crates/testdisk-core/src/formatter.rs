use crate::domain::{AccessCapability, DiskInfo, DiskKind, DiskSource, TargetSafety};
use log::info;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Unsupported platform: disk formatting is not available on this platform")]
    UnsupportedPlatform,
    #[error("Unsupported filesystem for current target: {0}")]
    UnsupportedFilesystem(String),
    #[error("Invalid disk target: {0}")]
    InvalidTarget(String),
    #[error("Refusing to format internal or unknown disk: {0}")]
    UnsafeTarget(String),
    #[error("Confirmation does not match target id")]
    ConfirmationMismatch,
    #[error("Invalid volume name")]
    InvalidVolumeName,
    #[error("Command unavailable: {0}")]
    CommandUnavailable(String),
    #[error("Format command failed during {step}: {stderr}")]
    CommandFailed {
        step: String,
        stdout: String,
        stderr: String,
    },
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
    Ntfs,
    Ext4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatFilesystemOption {
    pub value: FormatFilesystem,
    pub label: String,
    pub command_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDiskRequest {
    pub path: String,
    pub target: DiskInfo,
    pub filesystem: FormatFilesystem,
    pub volume_name: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatCommandStep {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOperationPlan {
    pub platform: String,
    pub target_path: String,
    pub target_id: String,
    pub filesystem: FormatFilesystem,
    pub volume_name: String,
    pub steps: Vec<FormatCommandStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDiskResult {
    pub disk_identifier: String,
    pub filesystem: FormatFilesystem,
    pub volume_name: String,
    pub message: String,
    pub status: String,
    pub stdout_summary: String,
    pub stderr_summary: String,
    pub post_check_status: String,
    pub plan: FormatOperationPlan,
}

pub fn format_filesystem_options(target: Option<&DiskInfo>) -> Vec<FormatFilesystemOption> {
    platform_supported_filesystems()
        .into_iter()
        .filter(|option| match target {
            Some(target) => target_supports_filesystem(target, &option.value),
            None => true,
        })
        .collect()
}

pub fn format_disk(request: FormatDiskRequest) -> Result<FormatDiskResult, FormatError> {
    let plan = build_format_plan(&request)?;
    execute_format_plan(plan)
}

pub fn preview_format_plan(
    request: &FormatDiskRequest,
) -> Result<FormatOperationPlan, FormatError> {
    build_format_plan(request)
}

fn build_format_plan(request: &FormatDiskRequest) -> Result<FormatOperationPlan, FormatError> {
    validate_target(&request.target)?;
    validate_confirmation(&request.confirmation, &request.target)?;
    let volume_name = validate_volume_name(&request.volume_name)?;

    if !target_supports_filesystem(&request.target, &request.filesystem) {
        return Err(FormatError::UnsupportedFilesystem(format!(
            "{:?}",
            request.filesystem
        )));
    }

    let target_id = format_target_identifier(&request.target)?;
    let target_path = request.target.path.clone();
    let filesystem_token = filesystem_command_token(&request.filesystem);

    Ok(FormatOperationPlan {
        platform: std::env::consts::OS.to_string(),
        target_path: target_path.clone(),
        target_id: target_id.clone(),
        filesystem: request.filesystem.clone(),
        volume_name: volume_name.clone(),
        steps: build_steps(
            &request.target,
            &target_id,
            &target_path,
            filesystem_token,
            &volume_name,
            &request.filesystem,
        )?,
    })
}

fn execute_format_plan(plan: FormatOperationPlan) -> Result<FormatDiskResult, FormatError> {
    let mut stdout_summary = Vec::new();
    let mut stderr_summary = Vec::new();

    for step in &plan.steps {
        info!(
            "format step={} program={} args={:?} target={}",
            step.name, step.program, step.args, plan.target_id
        );
        let output = run_command(step)?;
        if !output.status_success {
            let stderr = summarize_text(&output.stderr, 400);
            let stdout = summarize_text(&output.stdout, 400);
            return Err(FormatError::CommandFailed {
                step: step.name.clone(),
                stdout,
                stderr,
            });
        }
        stdout_summary.push(format!(
            "{}: {}",
            step.name,
            summarize_text(&output.stdout, 200)
        ));
        if !output.stderr.trim().is_empty() {
            stderr_summary.push(format!(
                "{}: {}",
                step.name,
                summarize_text(&output.stderr, 200)
            ));
        }
    }

    Ok(FormatDiskResult {
        disk_identifier: plan.target_id.clone(),
        filesystem: plan.filesystem.clone(),
        volume_name: plan.volume_name.clone(),
        message: format!(
            "{} -> {} on {} completed",
            plan.target_path, plan.volume_name, plan.platform
        ),
        status: "success".to_string(),
        stdout_summary: stdout_summary.join(" | "),
        stderr_summary: stderr_summary.join(" | "),
        post_check_status: "verified".to_string(),
        plan,
    })
}

struct CommandOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

fn run_command(step: &FormatCommandStep) -> Result<CommandOutput, FormatError> {
    let output = Command::new(&step.program)
        .args(&step.args)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FormatError::CommandUnavailable(step.program.clone())
            } else {
                FormatError::Io(error)
            }
        })?;

    Ok(CommandOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn build_steps(
    target: &DiskInfo,
    _target_id: &str,
    _target_path: &str,
    filesystem_token: &str,
    volume_name: &str,
    filesystem: &FormatFilesystem,
) -> Result<Vec<FormatCommandStep>, FormatError> {
    let _ = (target, filesystem);
    #[cfg(target_os = "macos")]
    {
        return Ok(vec![
            FormatCommandStep {
                name: "unmount".to_string(),
                program: "diskutil".to_string(),
                args: vec![
                    "unmountDisk".to_string(),
                    "force".to_string(),
                    _target_id.to_string(),
                ],
            },
            FormatCommandStep {
                name: "format".to_string(),
                program: "diskutil".to_string(),
                args: vec![
                    "eraseDisk".to_string(),
                    filesystem_token.to_string(),
                    volume_name.to_string(),
                    _target_id.to_string(),
                ],
            },
            FormatCommandStep {
                name: "verify".to_string(),
                program: "diskutil".to_string(),
                args: vec!["verifyDisk".to_string(), _target_id.to_string()],
            },
            FormatCommandStep {
                name: "rescan".to_string(),
                program: "diskutil".to_string(),
                args: vec![
                    "info".to_string(),
                    "-plist".to_string(),
                    _target_id.to_string(),
                ],
            },
        ]);
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(vec![
            FormatCommandStep {
                name: "unmount".to_string(),
                program: "umount".to_string(),
                args: vec![_target_path.to_string()],
            },
            FormatCommandStep {
                name: "format".to_string(),
                program: filesystem_token.to_string(),
                args: filesystem_args(filesystem, volume_name, _target_path),
            },
            FormatCommandStep {
                name: "verify".to_string(),
                program: "blkid".to_string(),
                args: vec![
                    "-o".to_string(),
                    "value".to_string(),
                    "-s".to_string(),
                    "TYPE".to_string(),
                    _target_path.to_string(),
                ],
            },
            FormatCommandStep {
                name: "rescan".to_string(),
                program: "partprobe".to_string(),
                args: vec![_target_path.to_string()],
            },
        ]);
    }
    #[cfg(target_os = "windows")]
    {
        let disk_number = windows_disk_number(target)?;
        return Ok(vec![
            FormatCommandStep {
                name: "unmount".to_string(),
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    format!("Set-Disk -Number {disk_number} -IsOffline $true -ErrorAction Stop"),
                ],
            },
            FormatCommandStep {
                name: "format".to_string(),
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    windows_format_script(disk_number, filesystem, volume_name),
                ],
            },
            FormatCommandStep {
                name: "verify".to_string(),
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    format!(
                        "Get-Volume -DiskNumber {disk_number} | Select-Object -ExpandProperty FileSystem"
                    ),
                ],
            },
            FormatCommandStep {
                name: "rescan".to_string(),
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    "Update-HostStorageCache".to_string(),
                ],
            },
        ]);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (filesystem_token, volume_name, filesystem);
        Err(FormatError::UnsupportedPlatform)
    }
}

fn validate_target(target: &DiskInfo) -> Result<(), FormatError> {
    if target.kind != DiskKind::Physical {
        return Err(FormatError::InvalidTarget(target.path.clone()));
    }
    if target.source != DiskSource::System {
        return Err(FormatError::UnsafeTarget(target.path.clone()));
    }
    if !matches!(
        target.safety,
        TargetSafety::External | TargetSafety::Removable
    ) {
        return Err(FormatError::UnsafeTarget(target.path.clone()));
    }
    if target.access != AccessCapability::ReadWrite || !target.readable || !target.writable {
        return Err(FormatError::UnsafeTarget(target.path.clone()));
    }
    Ok(())
}

fn validate_confirmation(confirmation: &str, target: &DiskInfo) -> Result<(), FormatError> {
    let expected = target.platform_id.trim();
    if expected.is_empty() {
        return Err(FormatError::InvalidTarget(target.path.clone()));
    }
    let confirmation = confirmation.trim();
    if confirmation == expected || confirmation == target.path.trim() {
        Ok(())
    } else {
        Err(FormatError::ConfirmationMismatch)
    }
}

fn validate_volume_name(name: &str) -> Result<String, FormatError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains(':')
        || trimmed.contains('\'')
        || trimmed.contains('"')
        || trimmed.contains('\\')
    {
        return Err(FormatError::InvalidVolumeName);
    }
    Ok(trimmed.to_string())
}

fn platform_supported_filesystems() -> Vec<FormatFilesystemOption> {
    #[cfg(target_os = "macos")]
    {
        return vec![
            FormatFilesystemOption {
                value: FormatFilesystem::Apfs,
                label: "APFS".to_string(),
                command_token: "APFS".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Exfat,
                label: "ExFAT".to_string(),
                command_token: "ExFAT".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Fat32,
                label: "MS-DOS FAT32".to_string(),
                command_token: "MS-DOS".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::HfsPlus,
                label: "Mac OS Extended (Journaled)".to_string(),
                command_token: "JHFS+".to_string(),
            },
        ];
    }
    #[cfg(target_os = "linux")]
    {
        return vec![
            FormatFilesystemOption {
                value: FormatFilesystem::Exfat,
                label: "ExFAT".to_string(),
                command_token: "mkfs.exfat".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Fat32,
                label: "FAT32".to_string(),
                command_token: "mkfs.fat".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Ext4,
                label: "ext4".to_string(),
                command_token: "mkfs.ext4".to_string(),
            },
        ];
    }
    #[cfg(target_os = "windows")]
    {
        return vec![
            FormatFilesystemOption {
                value: FormatFilesystem::Exfat,
                label: "ExFAT".to_string(),
                command_token: "ExFAT".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Fat32,
                label: "FAT32".to_string(),
                command_token: "FAT32".to_string(),
            },
            FormatFilesystemOption {
                value: FormatFilesystem::Ntfs,
                label: "NTFS".to_string(),
                command_token: "NTFS".to_string(),
            },
        ];
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        vec![]
    }
}

fn target_supports_filesystem(target: &DiskInfo, filesystem: &FormatFilesystem) -> bool {
    if !matches!(
        target.safety,
        TargetSafety::External | TargetSafety::Removable
    ) {
        return false;
    }
    if target.source != DiskSource::System || target.kind != DiskKind::Physical {
        return false;
    }
    if target.access != AccessCapability::ReadWrite {
        return false;
    }

    match filesystem {
        #[cfg(target_os = "macos")]
        FormatFilesystem::Apfs => true,
        #[cfg(target_os = "macos")]
        FormatFilesystem::HfsPlus => true,
        #[cfg(target_os = "macos")]
        FormatFilesystem::Exfat => true,
        #[cfg(target_os = "macos")]
        FormatFilesystem::Fat32 => true,

        #[cfg(target_os = "linux")]
        FormatFilesystem::Exfat => true,
        #[cfg(target_os = "linux")]
        FormatFilesystem::Fat32 => true,
        #[cfg(target_os = "linux")]
        FormatFilesystem::Ext4 => true,

        #[cfg(target_os = "windows")]
        FormatFilesystem::Exfat => true,
        #[cfg(target_os = "windows")]
        FormatFilesystem::Fat32 => true,
        #[cfg(target_os = "windows")]
        FormatFilesystem::Ntfs => true,

        _ => false,
    }
}

fn format_target_identifier(target: &DiskInfo) -> Result<String, FormatError> {
    #[cfg(target_os = "macos")]
    {
        return normalize_macos_disk_identifier(&target.platform_id, &target.path);
    }
    #[cfg(target_os = "linux")]
    {
        return normalize_linux_device_name(&target.path, &target.platform_id);
    }
    #[cfg(target_os = "windows")]
    {
        return normalize_windows_disk_number(&target.platform_id)
            .map(|number| format!("PhysicalDrive{number}"));
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = target;
        Err(FormatError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "macos")]
fn normalize_macos_disk_identifier(platform_id: &str, path: &str) -> Result<String, FormatError> {
    let candidate = platform_id
        .trim()
        .trim_start_matches("/dev/")
        .trim_start_matches("r");
    if candidate.starts_with("disk") && !candidate.contains('s') {
        return Ok(candidate.to_string());
    }

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

#[cfg(target_os = "linux")]
fn normalize_linux_device_name(path: &str, platform_id: &str) -> Result<String, FormatError> {
    let candidate = platform_id
        .trim()
        .strip_prefix("/dev/")
        .unwrap_or(platform_id.trim());
    if !candidate.is_empty() {
        return Ok(candidate.to_string());
    }

    let trimmed = path.trim();
    let identifier = trimmed
        .strip_prefix("/dev/")
        .ok_or_else(|| FormatError::InvalidTarget(path.to_string()))?;
    if identifier.is_empty() {
        return Err(FormatError::InvalidTarget(path.to_string()));
    }
    Ok(identifier.to_string())
}

#[cfg(target_os = "windows")]
fn normalize_windows_disk_number(platform_id: &str) -> Result<u32, FormatError> {
    let cleaned = platform_id
        .trim()
        .trim_start_matches("\\\\.\\PhysicalDrive")
        .trim_start_matches("physicaldrive")
        .trim_start_matches("PhysicalDrive")
        .trim();
    cleaned
        .parse::<u32>()
        .map_err(|_| FormatError::InvalidTarget(platform_id.to_string()))
}

fn filesystem_command_token(filesystem: &FormatFilesystem) -> &'static str {
    match filesystem {
        #[cfg(target_os = "macos")]
        FormatFilesystem::Apfs => "APFS",
        #[cfg(target_os = "macos")]
        FormatFilesystem::Exfat => "ExFAT",
        #[cfg(target_os = "macos")]
        FormatFilesystem::Fat32 => "MS-DOS",
        #[cfg(target_os = "macos")]
        FormatFilesystem::HfsPlus => "JHFS+",

        #[cfg(target_os = "linux")]
        FormatFilesystem::Exfat => "mkfs.exfat",
        #[cfg(target_os = "linux")]
        FormatFilesystem::Fat32 => "mkfs.fat",
        #[cfg(target_os = "linux")]
        FormatFilesystem::Ext4 => "mkfs.ext4",

        #[cfg(target_os = "windows")]
        FormatFilesystem::Exfat => "ExFAT",
        #[cfg(target_os = "windows")]
        FormatFilesystem::Fat32 => "FAT32",
        #[cfg(target_os = "windows")]
        FormatFilesystem::Ntfs => "NTFS",

        _ => "",
    }
}

#[cfg(target_os = "windows")]
fn windows_disk_number(target: &DiskInfo) -> Result<u32, FormatError> {
    normalize_windows_disk_number(&target.platform_id)
        .or_else(|_| normalize_windows_disk_number(&target.path))
}

#[cfg(target_os = "linux")]
fn filesystem_args(
    filesystem: &FormatFilesystem,
    volume_name: &str,
    target_path: &str,
) -> Vec<String> {
    match filesystem {
        FormatFilesystem::Exfat => vec![
            "-n".to_string(),
            volume_name.to_string(),
            target_path.to_string(),
        ],
        FormatFilesystem::Fat32 => vec![
            "-F".to_string(),
            "32".to_string(),
            "-n".to_string(),
            volume_name.to_string(),
            target_path.to_string(),
        ],
        FormatFilesystem::Ext4 => vec![
            "-F".to_string(),
            "-L".to_string(),
            volume_name.to_string(),
            target_path.to_string(),
        ],
        FormatFilesystem::HfsPlus => vec![
            "-v".to_string(),
            volume_name.to_string(),
            target_path.to_string(),
        ],
        _ => vec![volume_name.to_string(), target_path.to_string()],
    }
}

#[cfg(target_os = "windows")]
fn windows_format_script(
    disk_number: u32,
    filesystem: &FormatFilesystem,
    volume_name: &str,
) -> String {
    let fs_name = match filesystem {
        FormatFilesystem::Exfat => "exFAT",
        FormatFilesystem::Fat32 => "FAT32",
        FormatFilesystem::Ntfs => "NTFS",
        _ => "exFAT",
    };

    format!(
        "$disk = Get-Disk -Number {disk_number}; \
         if ($disk.IsOffline) {{ Set-Disk -Number {disk_number} -IsOffline $false -ErrorAction SilentlyContinue }}; \
         Initialize-Disk -Number {disk_number} -PartitionStyle GPT -ErrorAction SilentlyContinue | Out-Null; \
         $partition = New-Partition -DiskNumber {disk_number} -UseMaximumSize -AssignDriveLetter -ErrorAction Stop; \
         Format-Volume -Partition $partition -FileSystem {fs_name} -NewFileSystemLabel '{volume_name}' -Confirm:$false -ErrorAction Stop | Out-Null"
    )
}

fn summarize_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        let mut out = String::new();
        let _ = write!(&mut out, "{}...", &trimmed[..max_len.saturating_sub(3)]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_target() -> DiskInfo {
        DiskInfo {
            path: "/dev/disk4".to_string(),
            display_path: "/dev/disk4".to_string(),
            raw_path: "/dev/rdisk4".to_string(),
            platform_id: "disk4".to_string(),
            name: "USB".to_string(),
            size_bytes: 1024 * 1024 * 1024,
            readable: true,
            writable: true,
            source: DiskSource::System,
            kind: DiskKind::Physical,
            safety: TargetSafety::External,
            access: AccessCapability::ReadWrite,
            protocol: Some("usb".to_string()),
            is_internal: Some(false),
            is_removable: Some(true),
        }
    }

    #[test]
    fn filters_unsupported_filesystems_for_internal_targets() {
        let mut target = sample_target();
        target.safety = TargetSafety::Internal;
        let options = format_filesystem_options(Some(&target));
        assert!(options.is_empty());
    }

    #[test]
    fn exposes_supported_platform_filesystems_for_external_targets() {
        let options = format_filesystem_options(Some(&sample_target()));
        assert!(!options.is_empty());
        assert!(options.iter().all(|option| option.label.len() > 0));
    }

    #[test]
    fn rejects_partition_confirmation_mismatch() {
        let mut request = FormatDiskRequest {
            path: "/dev/disk4".to_string(),
            target: sample_target(),
            filesystem: FormatFilesystem::Exfat,
            volume_name: "USB".to_string(),
            confirmation: "disk5".to_string(),
        };

        let err = build_format_plan(&request).unwrap_err();
        assert!(matches!(err, FormatError::ConfirmationMismatch));

        request.confirmation = "disk4".to_string();
        let plan = build_format_plan(&request);
        assert!(plan.is_ok());
    }

    #[test]
    fn previews_format_plan_without_execution() {
        let request = FormatDiskRequest {
            path: "/dev/disk4".to_string(),
            target: sample_target(),
            filesystem: FormatFilesystem::Exfat,
            volume_name: "USB".to_string(),
            confirmation: "disk4".to_string(),
        };

        let plan = preview_format_plan(&request).unwrap();
        assert_eq!(plan.target_id, "disk4");
        assert!(!plan.steps.is_empty());
    }
}
