use crate::disk::{DiskError, DiskReader};
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

pub const RECOVERY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("disk error: {0}")]
    Disk(#[from] DiskError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported filesystem: {0}")]
    UnsupportedFilesystem(String),
    #[error("invalid destination: {0}")]
    InvalidDestination(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveredFileRecord {
    pub schema_version: u32,
    pub source_path: String,
    pub destination_path: String,
    pub deleted: bool,
    pub status: String,
    pub bytes_written: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryJobTranscript {
    pub schema_version: u32,
    pub source_path: String,
    pub destination_dir: String,
    pub recovered_files: Vec<RecoveredFileRecord>,
    pub status: String,
}

pub fn recover_fat_deleted_files(
    source_path: &str,
    destination_dir: &str,
) -> Result<RecoveryJobTranscript, RecoveryError> {
    let source = std::path::Path::new(source_path);
    let destination = std::path::Path::new(destination_dir);
    if source_path == destination_dir {
        return Err(RecoveryError::InvalidDestination(
            "destination must differ from source".to_string(),
        ));
    }
    fs::create_dir_all(destination)?;
    let mut disk = DiskReader::open(source)?;
    let boot = disk.read_sector(0)?;
    let bytes_per_sector = u16::from_le_bytes([boot[11], boot[12]]) as u64;
    let sectors_per_cluster = boot[13] as u64;
    let reserved = u16::from_le_bytes([boot[14], boot[15]]) as u64;
    let fats = boot[16] as u64;
    let root_entry_count = u16::from_le_bytes([boot[17], boot[18]]) as u64;
    let sectors_per_fat = u16::from_le_bytes([boot[22], boot[23]]) as u64;
    let root_dir_start = reserved + fats * sectors_per_fat;
    let root_dir_sectors = ((root_entry_count * 32) + (bytes_per_sector.saturating_sub(1))) / bytes_per_sector;
    let root_dir_offset = root_dir_start * bytes_per_sector;
    let root_dir = disk.read_at(root_dir_offset, (root_dir_sectors * bytes_per_sector) as usize)?;

    let mut recovered_files = Vec::new();
    for chunk in root_dir.chunks_exact(32) {
        if chunk[0] == 0x00 {
            break;
        }
        if chunk[0] != 0xE5 {
            continue;
        }
        let attrs = chunk[11];
        if attrs & 0x0F == 0x0F {
            continue;
        }
        let name = sanitize_fat_name(&chunk[1..11]);
        let ext = sanitize_fat_name(&chunk[11 - 3..11 - 0]);
        let file_name = if ext.is_empty() {
            name.clone()
        } else {
            format!("{name}.{ext}")
        };
        let first_cluster = u16::from_le_bytes([chunk[26], chunk[27]]) as u64;
        let file_size = u32::from_le_bytes([chunk[28], chunk[29], chunk[30], chunk[31]]) as u64;
        if first_cluster == 0 || file_size == 0 {
            continue;
        }

        let data_region = root_dir_start + root_dir_sectors + fats * sectors_per_fat;
        let cluster_offset = (data_region + (first_cluster.saturating_sub(2)) * sectors_per_cluster)
            * bytes_per_sector;
        let bytes = disk.read_at(cluster_offset, file_size as usize)?;
        let out_path = destination.join(if file_name.is_empty() {
            format!("recovered-{}.bin", recovered_files.len() + 1)
        } else {
            file_name
        });
        fs::write(&out_path, &bytes)?;
        recovered_files.push(RecoveredFileRecord {
            schema_version: RECOVERY_SCHEMA_VERSION,
            source_path: source_path.to_string(),
            destination_path: out_path.display().to_string(),
            deleted: true,
            status: "recovered".to_string(),
            bytes_written: bytes.len() as u64,
            note: "FAT metadata recovery".to_string(),
        });
    }

    Ok(RecoveryJobTranscript {
        schema_version: RECOVERY_SCHEMA_VERSION,
        source_path: source_path.to_string(),
        destination_dir: destination_dir.to_string(),
        recovered_files,
        status: "completed".to_string(),
    })
}

fn sanitize_fat_name(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes)
        .replace(char::from(0x20), "")
        .trim()
        .to_string();
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

pub fn export_recovery_json(transcript: &RecoveryJobTranscript) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(transcript)
}

pub fn import_recovery_json(json: &str) -> Result<RecoveryJobTranscript, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_same_source_and_destination() {
        let err = recover_fat_deleted_files("/tmp/a.img", "/tmp/a.img").unwrap_err();
        assert!(matches!(err, RecoveryError::InvalidDestination(_)));
    }
}
