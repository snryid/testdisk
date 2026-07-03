use crate::disk::DiskError;
use crate::domain::{AccessCapability, DiskInfo, DiskKind, DiskSource, PartitionTableType, TargetSafety};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

pub const WRITEBACK_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum WritebackError {
    #[error("unsafe target: {0}")]
    UnsafeTarget(String),
    #[error("invalid plan: {0}")]
    InvalidPlan(String),
    #[error("patch mismatch at LBA {0}")]
    PatchMismatch(u64),
    #[error("backup missing: {0}")]
    BackupMissing(String),
    #[error("disk error: {0}")]
    Disk(#[from] DiskError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WritebackSectorPatch {
    pub lba: u64,
    pub before_hex: String,
    pub after_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WritebackPlan {
    pub schema_version: u32,
    pub target_path: String,
    pub target_platform_id: String,
    pub target_signature: String,
    pub backup_path: String,
    pub partition_table_type: PartitionTableType,
    pub sector_size_bytes: u64,
    pub patches: Vec<WritebackSectorPatch>,
    pub guard_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WritebackStepRecord {
    pub lba: u64,
    pub bytes_written: u64,
    pub checksum_before: String,
    pub checksum_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WritebackTranscript {
    pub schema_version: u32,
    pub target_path: String,
    pub backup_path: String,
    pub bytes_written: u64,
    pub checksum: String,
    pub steps: Vec<WritebackStepRecord>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreTranscript {
    pub schema_version: u32,
    pub target_path: String,
    pub backup_path: String,
    pub restored_sectors: Vec<u64>,
    pub checksum: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BackupSector {
    lba: u64,
    bytes_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BackupManifest {
    schema_version: u32,
    target_signature: String,
    sector_size_bytes: u64,
    sectors: Vec<BackupSector>,
}

pub fn build_guarded_writeback_plan(
    target: &DiskInfo,
    partition_table_type: PartitionTableType,
    backup_path: impl Into<String>,
    patches: Vec<WritebackSectorPatch>,
) -> Result<WritebackPlan, WritebackError> {
    validate_target(target)?;
    if patches.is_empty() {
        return Err(WritebackError::InvalidPlan(
            "write-back plan must include at least one patch".to_string(),
        ));
    }

    let mut normalized_patches = patches;
    normalized_patches.sort_by(|left, right| left.lba.cmp(&right.lba));

    let sector_size_bytes = normalized_patches
        .first()
        .map(|patch| hex_decode(&patch.after_hex).map(|bytes| bytes.len() as u64))
        .transpose()?
        .ok_or_else(|| WritebackError::InvalidPlan("missing first patch".to_string()))?;

    for patch in &normalized_patches {
        let before = hex_decode(&patch.before_hex)?;
        let after = hex_decode(&patch.after_hex)?;
        if before.len() != after.len() || before.is_empty() {
            return Err(WritebackError::InvalidPlan(format!(
                "patch at LBA {} must have equal non-empty before/after bytes",
                patch.lba
            )));
        }
        if before.len() as u64 != sector_size_bytes {
            return Err(WritebackError::InvalidPlan(format!(
                "patch at LBA {} does not match the expected sector size",
                patch.lba
            )));
        }
    }

    let backup_path = backup_path.into();
    if backup_path.trim().is_empty() {
        return Err(WritebackError::InvalidPlan(
            "backup path must not be empty".to_string(),
        ));
    }

    Ok(WritebackPlan {
        schema_version: WRITEBACK_SCHEMA_VERSION,
        target_path: target.path.clone(),
        target_platform_id: target.platform_id.clone(),
        target_signature: target_signature(target),
        backup_path,
        partition_table_type,
        sector_size_bytes,
        patches: normalized_patches,
        guard_notes: vec![
            format!("target_safety={:?}", target.safety),
            format!("target_access={:?}", target.access),
        ],
    })
}

pub fn apply_writeback_plan(plan: &WritebackPlan) -> Result<WritebackTranscript, WritebackError> {
    let mut file = open_target_rw(&plan.target_path)?;
    let mut backup = BackupManifest {
        schema_version: WRITEBACK_SCHEMA_VERSION,
        target_signature: plan.target_signature.clone(),
        sector_size_bytes: plan.sector_size_bytes,
        sectors: Vec::new(),
    };
    let mut steps = Vec::new();
    let mut bytes_written = 0u64;
    let mut hash = Hasher::new();

    for patch in &plan.patches {
        let expected_before = hex_decode(&patch.before_hex)?;
        let _replacement = hex_decode(&patch.after_hex)?;
        let offset = patch.lba.saturating_mul(plan.sector_size_bytes);
        let current = read_at(&mut file, offset, expected_before.len())?;
        if current != expected_before {
            return Err(WritebackError::PatchMismatch(patch.lba));
        }
        backup.sectors.push(BackupSector {
            lba: patch.lba,
            bytes_hex: patch.before_hex.clone(),
        });
    }

    write_backup_manifest(Path::new(&plan.backup_path), &backup)?;

    for patch in &plan.patches {
        let replacement = hex_decode(&patch.after_hex)?;
        let offset = patch.lba.saturating_mul(plan.sector_size_bytes);
        write_at(&mut file, offset, &replacement)?;
        let reread = read_at(&mut file, offset, replacement.len())?;
        if reread != replacement {
            return Err(WritebackError::PatchMismatch(patch.lba));
        }
        hash.update(&reread);
        bytes_written += reread.len() as u64;
        steps.push(WritebackStepRecord {
            lba: patch.lba,
            bytes_written: reread.len() as u64,
            checksum_before: checksum_hex(&hex_decode(&patch.before_hex)?),
            checksum_after: checksum_hex(&reread),
        });
    }

    Ok(WritebackTranscript {
        schema_version: WRITEBACK_SCHEMA_VERSION,
        target_path: plan.target_path.clone(),
        backup_path: plan.backup_path.clone(),
        bytes_written,
        checksum: format!("{:08x}", hash.finalize()),
        steps,
        status: "verified".to_string(),
    })
}

pub fn restore_writeback_backup(
    plan: &WritebackPlan,
) -> Result<RestoreTranscript, WritebackError> {
    let manifest = read_backup_manifest(Path::new(&plan.backup_path))?;
    if manifest.target_signature != plan.target_signature {
        return Err(WritebackError::InvalidPlan(
            "backup does not match target signature".to_string(),
        ));
    }

    let mut file = open_target_rw(&plan.target_path)?;
    let mut restored = Vec::new();
    let mut hash = Hasher::new();

    for sector in &manifest.sectors {
        let bytes = hex_decode(&sector.bytes_hex)?;
        let offset = sector.lba.saturating_mul(manifest.sector_size_bytes);
        write_at(&mut file, offset, &bytes)?;
        let reread = read_at(&mut file, offset, bytes.len())?;
        if reread != bytes {
            return Err(WritebackError::PatchMismatch(sector.lba));
        }
        restored.push(sector.lba);
        hash.update(&reread);
    }

    Ok(RestoreTranscript {
        schema_version: WRITEBACK_SCHEMA_VERSION,
        target_path: plan.target_path.clone(),
        backup_path: plan.backup_path.clone(),
        restored_sectors: restored,
        checksum: format!("{:08x}", hash.finalize()),
        status: "restored".to_string(),
    })
}

fn validate_target(target: &DiskInfo) -> Result<(), WritebackError> {
    if target.kind != DiskKind::Image || target.source != DiskSource::ImageFile {
        return Err(WritebackError::UnsafeTarget(target.path.clone()));
    }
    if target.safety != TargetSafety::Image {
        return Err(WritebackError::UnsafeTarget(target.path.clone()));
    }
    if target.access != AccessCapability::ReadWrite || !target.readable || !target.writable {
        return Err(WritebackError::UnsafeTarget(target.path.clone()));
    }
    Ok(())
}

fn target_signature(target: &DiskInfo) -> String {
    format!("{}:{}:{}", target.path, target.size_bytes, target.platform_id)
}

fn open_target_rw(path: &str) -> Result<File, WritebackError> {
    Ok(OpenOptions::new().read(true).write(true).open(path)?)
}

fn read_at(file: &mut File, offset: u64, len: usize) -> Result<Vec<u8>, WritebackError> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; len];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn write_at(file: &mut File, offset: u64, bytes: &[u8]) -> Result<(), WritebackError> {
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(())
}

fn write_backup_manifest(path: &Path, manifest: &BackupManifest) -> Result<(), WritebackError> {
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(path, json)?;
    Ok(())
}

fn read_backup_manifest(path: &Path) -> Result<BackupManifest, WritebackError> {
    let json = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            WritebackError::BackupMissing(path.display().to_string())
        } else {
            WritebackError::Io(error)
        }
    })?;
    Ok(serde_json::from_str(&json)?)
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, WritebackError> {
    let trimmed = hex.trim();
    if trimmed.len() % 2 != 0 {
        return Err(WritebackError::InvalidPlan(
            "hex payload must have an even number of characters".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    let chars: Vec<char> = trimmed.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        let high = chars[i].to_digit(16).ok_or_else(|| {
            WritebackError::InvalidPlan("invalid hex digit in payload".to_string())
        })?;
        let low = chars[i + 1].to_digit(16).ok_or_else(|| {
            WritebackError::InvalidPlan("invalid hex digit in payload".to_string())
        })?;
        bytes.push(((high << 4) | low) as u8);
    }
    Ok(bytes)
}

fn checksum_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    format!("{:08x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_target(path: &Path) -> DiskInfo {
        DiskInfo {
            path: path.display().to_string(),
            display_path: path.display().to_string(),
            raw_path: path.display().to_string(),
            platform_id: "image-001".to_string(),
            name: "fixture".to_string(),
            size_bytes: 1024,
            readable: true,
            writable: true,
            source: DiskSource::ImageFile,
            kind: DiskKind::Image,
            safety: TargetSafety::Image,
            access: AccessCapability::ReadWrite,
            protocol: None,
            is_internal: None,
            is_removable: None,
        }
    }

    fn write_fixture(path: &Path) -> Vec<u8> {
        let mut bytes = vec![0u8; 1024];
        bytes[..16].copy_from_slice(b"ORIGINAL-SECTOR_");
        fs::write(path, &bytes).unwrap();
        bytes
    }

    #[test]
    fn rejects_unsafe_physical_targets() {
        let target = DiskInfo {
            path: "/dev/disk9".to_string(),
            display_path: "/dev/disk9".to_string(),
            raw_path: "/dev/rdisk9".to_string(),
            platform_id: "disk9".to_string(),
            name: "physical".to_string(),
            size_bytes: 1024,
            readable: true,
            writable: true,
            source: DiskSource::System,
            kind: DiskKind::Physical,
            safety: TargetSafety::External,
            access: AccessCapability::ReadWrite,
            protocol: None,
            is_internal: Some(false),
            is_removable: Some(true),
        };

        let patch = WritebackSectorPatch {
            lba: 0,
            before_hex: "00".to_string(),
            after_hex: "00".to_string(),
        };

        let err = build_guarded_writeback_plan(
            &target,
            PartitionTableType::Mbr,
            "/tmp/backup.json",
            vec![patch],
        )
        .unwrap_err();

        assert!(matches!(err, WritebackError::UnsafeTarget(_)));
    }

    #[test]
    fn applies_and_restores_sector_patch_with_transcript() {
        let dir = std::env::temp_dir();
        let image_path = dir.join("testdisk-writeback.img");
        let backup_path = dir.join("testdisk-writeback-backup.json");
        let original = write_fixture(&image_path);

        let target = sample_target(&image_path);
        let patch = WritebackSectorPatch {
            lba: 0,
            before_hex: hex_encode(&original),
            after_hex: {
                let mut bytes = original.clone();
                bytes[..16].copy_from_slice(b"UPDATED-SECTOR__");
                hex_encode(&bytes)
            },
        };

        let plan = build_guarded_writeback_plan(
            &target,
            PartitionTableType::Mbr,
            backup_path.to_string_lossy().to_string(),
            vec![patch],
        )
        .unwrap();
        let transcript = apply_writeback_plan(&plan).unwrap();
        assert_eq!(transcript.status, "verified");
        let updated = fs::read(&image_path).unwrap();
        assert_eq!(&updated[..16], b"UPDATED-SECTOR__");

        let restore = restore_writeback_backup(&plan).unwrap();
        assert_eq!(restore.status, "restored");
        let restored = fs::read(&image_path).unwrap();
        assert_eq!(&restored[..16], b"ORIGINAL-SECTOR_");

        let _ = fs::remove_file(image_path);
        let _ = fs::remove_file(backup_path);
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
