use crate::disk::{DiskError, DiskReader, SECTOR_SIZE};
use crate::fs_detect::detect_filesystem;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const BOOT_SECTOR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum BootSectorError {
    #[error("disk error: {0}")]
    Disk(#[from] DiskError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported filesystem: {0}")]
    UnsupportedFilesystem(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeRepairCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootSectorDiagnosis {
    pub schema_version: u32,
    pub target_path: String,
    pub filesystem: String,
    pub primary_lba: u64,
    pub backup_lba: Option<u64>,
    pub matches_backup: bool,
    pub confidence: String,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootSectorRepairPlan {
    pub schema_version: u32,
    pub target_path: String,
    pub filesystem: String,
    pub primary_lba: u64,
    pub backup_lba: Option<u64>,
    pub native_commands: Vec<NativeRepairCommand>,
    pub notes: Vec<String>,
}

pub fn diagnose_boot_sectors(path: &str) -> Result<Vec<BootSectorDiagnosis>, BootSectorError> {
    let mut disk = DiskReader::open(path)?;
    let primary = disk.read_sector(0)?;
    let mut findings = Vec::new();

    if let Some(info) = detect_filesystem(&primary) {
        let filesystem = info.fs_type;
        let backup_lba = backup_sector_for(&filesystem, &mut disk)?;
        let matches_backup = if let Some(backup_lba) = backup_lba {
            let backup = disk.read_sector(backup_lba)?;
            backup == primary
        } else {
            false
        };
        if matches_backup {
            findings.push("primary boot sector matches backup copy".to_string());
        } else {
            findings.push("primary boot sector differs from backup copy".to_string());
        }

        return Ok(vec![BootSectorDiagnosis {
            schema_version: BOOT_SECTOR_SCHEMA_VERSION,
            target_path: path.to_string(),
            filesystem,
            primary_lba: 0,
            backup_lba,
            matches_backup,
            confidence: info.confidence,
            findings,
        }]);
    }

    Ok(vec![BootSectorDiagnosis {
        schema_version: BOOT_SECTOR_SCHEMA_VERSION,
        target_path: path.to_string(),
        filesystem: "unknown".to_string(),
        primary_lba: 0,
        backup_lba: None,
        matches_backup: false,
        confidence: "low".to_string(),
        findings: vec!["no supported boot sector signature detected".to_string()],
    }])
}

pub fn preview_boot_sector_repair(
    path: &str,
) -> Result<Vec<BootSectorRepairPlan>, BootSectorError> {
    let diagnoses = diagnose_boot_sectors(path)?;
    let mut plans = Vec::new();
    for diagnosis in diagnoses {
        if diagnosis.filesystem == "unknown" {
            continue;
        }
        plans.push(BootSectorRepairPlan {
            schema_version: BOOT_SECTOR_SCHEMA_VERSION,
            target_path: diagnosis.target_path.clone(),
            filesystem: diagnosis.filesystem.clone(),
            primary_lba: diagnosis.primary_lba,
            backup_lba: diagnosis.backup_lba,
            native_commands: native_commands_for(&diagnosis.filesystem, &diagnosis.target_path),
            notes: vec![
                "read-only diagnosis completed".to_string(),
                "repair plans are preview-only until user confirmation".to_string(),
            ],
        });
    }
    Ok(plans)
}

fn backup_sector_for(filesystem: &str, disk: &mut DiskReader) -> Result<Option<u64>, BootSectorError> {
    let size_lba = disk.size / SECTOR_SIZE;
    let backup = match filesystem {
        "FAT12" | "FAT16" | "FAT32" | "exFAT" | "NTFS" => Some(6),
        "ext2" | "ext3" | "ext4" => Some(2),
        "HFS" | "HFS+" | "APFS" => size_lba.checked_sub(1),
        _ => None,
    };
    Ok(backup.filter(|lba| *lba < size_lba))
}

fn native_commands_for(filesystem: &str, target_path: &str) -> Vec<NativeRepairCommand> {
    match filesystem {
        "FAT12" | "FAT16" | "FAT32" => vec![NativeRepairCommand {
            program: "fsck.fat".to_string(),
            args: vec!["-n".to_string(), target_path.to_string()],
        }],
        "NTFS" => vec![NativeRepairCommand {
            program: "ntfsfix".to_string(),
            args: vec!["-n".to_string(), target_path.to_string()],
        }],
        "ext2" | "ext3" | "ext4" => vec![NativeRepairCommand {
            program: "fsck".to_string(),
            args: vec!["-n".to_string(), target_path.to_string()],
        }],
        "HFS" | "HFS+" | "APFS" => vec![NativeRepairCommand {
            program: "diskutil".to_string(),
            args: vec!["repairVolume".to_string(), target_path.to_string()],
        }],
        _ => vec![],
    }
}

pub fn export_boot_sector_json(plan: &[BootSectorRepairPlan]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(plan)
}

pub fn import_boot_sector_json(json: &str) -> Result<Vec<BootSectorRepairPlan>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, BootSectorError> {
    Ok(fs::read(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fixture(path: &Path) {
        let mut bytes = vec![0u8; 4096];
        bytes[3..11].copy_from_slice(b"NTFS    ");
        bytes[11..13].copy_from_slice(&512u16.to_le_bytes());
        bytes[510..512].copy_from_slice(&0xAA55u16.to_le_bytes());
        let primary = bytes[..512].to_vec();
        bytes[512 * 6..512 * 6 + 512].copy_from_slice(&primary);
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn diagnoses_matching_ntfs_boot_sector() {
        let path = std::env::temp_dir().join("testdisk-boot-sector.img");
        write_fixture(&path);
        let diagnosis = diagnose_boot_sectors(path.to_str().unwrap()).unwrap();
        assert_eq!(diagnosis[0].filesystem, "NTFS");
        assert!(diagnosis[0].matches_backup);
        let _ = fs::remove_file(path);
    }
}
