use crate::disk::{DiskError, DiskReader, SECTOR_SIZE};
use crate::fs_detect::{detect_filesystem, FilesystemInfo};
use crate::gpt::{detect_gpt, GptTable};
use crate::mbr::{is_gpt_protective_mbr, read_mbr, MbrTable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionTableType {
    Gpt,
    Mbr,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionResult {
    pub index: u32,
    pub name: String,
    pub start_lba: u64,
    pub end_lba: u64,
    pub size_bytes: u64,
    pub type_name: String,
    pub filesystem: Option<FilesystemInfo>,
    pub status: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub disk_path: String,
    pub disk_size: u64,
    pub partition_table_type: PartitionTableType,
    pub mbr: Option<MbrTable>,
    pub gpt: Option<GptTable>,
    pub partitions: Vec<PartitionResult>,
    pub lost_partitions: Vec<PartitionResult>,
    pub warnings: Vec<String>,
}

/// Main scan — mirrors TestDisk's autodetect_arch + read_part flow.
pub fn scan_disk(path: &str) -> Result<ScanResult, DiskError> {
    let mut disk = DiskReader::open(path)?;
    let mut warnings = Vec::new();

    let mbr = read_mbr(&mut disk).ok();
    let gpt = detect_gpt(&mut disk)?;

    let (table_type, partitions) = if let Some(ref gpt_table) = gpt {
        if !gpt_table.crc_valid {
            warnings.push("GPT 头 CRC 校验失败，分区表可能已损坏".to_string());
        }
        let parts = gpt_table
            .partitions
            .iter()
            .map(|p| {
                let mut part = PartitionResult {
                    index: p.index,
                    name: if p.name.is_empty() {
                        p.type_name.clone()
                    } else {
                        p.name.clone()
                    },
                    start_lba: p.start_lba,
                    end_lba: p.end_lba,
                    size_bytes: p.size_bytes,
                    type_name: p.type_name.clone(),
                    filesystem: None,
                    status: "primary".to_string(),
                    source: "gpt".to_string(),
                };
                part.filesystem = detect_fs_at(&mut disk, p.start_lba * SECTOR_SIZE);
                part
            })
            .collect();
        (PartitionTableType::Gpt, parts)
    } else if let Some(ref mbr_table) = mbr {
        if !mbr_table.signature_valid {
            warnings.push("MBR 签名无效 (0x55AA)".to_string());
        }
        if is_gpt_protective_mbr(mbr_table) {
            warnings.push("检测到 GPT 保护性 MBR，但 GPT 头未找到".to_string());
        }
        let parts = mbr_table
            .partitions
            .iter()
            .map(|p| {
                let end_lba = p.start_lba + p.sector_count.saturating_sub(1);
                let mut part = PartitionResult {
                    index: p.index as u32,
                    name: format!("Partition {}", p.index),
                    start_lba: p.start_lba,
                    end_lba,
                    size_bytes: p.size_bytes,
                    type_name: p.type_name.clone(),
                    filesystem: None,
                    status: if p.bootable {
                        "bootable".to_string()
                    } else {
                        "primary".to_string()
                    },
                    source: "mbr".to_string(),
                };
                part.filesystem = detect_fs_at(&mut disk, p.start_lba * SECTOR_SIZE);
                part
            })
            .collect();
        (PartitionTableType::Mbr, parts)
    } else {
        warnings.push("未检测到有效的分区表".to_string());
        (PartitionTableType::Unknown, vec![])
    };

    // Quick lost partition search — scan for filesystem signatures at 1MB boundaries
    let lost = search_lost_partitions(&mut disk, &partitions);

    Ok(ScanResult {
        disk_path: disk.path.clone(),
        disk_size: disk.size,
        partition_table_type: table_type,
        mbr,
        gpt,
        partitions,
        lost_partitions: lost,
        warnings,
    })
}

fn detect_fs_at(disk: &mut DiskReader, offset: u64) -> Option<FilesystemInfo> {
    // Read 8KB to cover ext/APFS/HFS superblock offsets
    let data = disk.read_at(offset, 8192).ok()?;
    detect_filesystem(&data)
}

/// Search for filesystem boot sectors not covered by partition table (simplified Deeper Search).
fn search_lost_partitions(
    disk: &mut DiskReader,
    known: &[PartitionResult],
) -> Vec<PartitionResult> {
    let mut found = Vec::new();
    let step = 2048u64; // 1MB alignment
    let max_lba = disk.size / SECTOR_SIZE;
    let mut lba = step;

    while lba < max_lba.saturating_sub(step) {
        let offset = lba * SECTOR_SIZE;
        if is_covered_by_known(offset, known) {
            lba += step;
            continue;
        }

        if let Ok(data) = disk.read_at(offset, 8192) {
            if let Some(fs) = detect_filesystem(&data) {
                let size_estimate = estimate_fs_size(&data, fs.fs_type.as_str());
                found.push(PartitionResult {
                    index: found.len() as u32 + 1,
                    name: format!("Lost: {}", fs.fs_type),
                    start_lba: lba,
                    end_lba: lba + size_estimate / SECTOR_SIZE,
                    size_bytes: size_estimate,
                    type_name: "Lost/Deleted".to_string(),
                    filesystem: Some(fs),
                    status: "deleted".to_string(),
                    source: "deep_scan".to_string(),
                });
                // Skip ahead to avoid duplicate hits
                lba += size_estimate / SECTOR_SIZE;
                continue;
            }
        }
        lba += step;
    }

    found
}

fn is_covered_by_known(offset: u64, known: &[PartitionResult]) -> bool {
    known.iter().any(|p| {
        let start = p.start_lba * SECTOR_SIZE;
        let end = (p.end_lba + 1) * SECTOR_SIZE;
        offset >= start && offset < end
    })
}

fn estimate_fs_size(data: &[u8], fs_type: &str) -> u64 {
    match fs_type {
        "NTFS" | "FAT32" | "FAT16" | "exFAT" => {
            if data.len() >= 32 {
                let sectors = u32::from_le_bytes([data[19], data[20], data[21], data[22]]);
                if sectors > 0 {
                    return (sectors as u64) * SECTOR_SIZE;
                }
            }
            100 * 1024 * 1024 // 100MB default
        }
        "ext2" | "ext3" | "ext4" => {
            if data.len() >= 1088 {
                let sb = &data[1024..];
                let blocks = u32::from_le_bytes([sb[4], sb[5], sb[6], sb[7]]);
                let block_size = 1024u64 << sb[24];
                return (blocks as u64) * block_size;
            }
            500 * 1024 * 1024
        }
        _ => 50 * 1024 * 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mbr_image(path: &str) {
        let mut img = vec![0u8; 4 * 1024 * 1024]; // 4MB image
        // MBR signature
        img[510] = 0x55;
        img[511] = 0xAA;
        // Partition 1: FAT32 at LBA 2048, 2048 sectors
        let entry_offset = 446;
        img[entry_offset + 4] = 0x0c; // FAT32 LBA
        img[entry_offset + 8..entry_offset + 12].copy_from_slice(&2048u32.to_le_bytes());
        img[entry_offset + 12..entry_offset + 16].copy_from_slice(&2048u32.to_le_bytes());
        // FAT boot sector at LBA 2048
        let fat_offset = 2048 * 512;
        img[fat_offset + 3..fat_offset + 11].copy_from_slice(b"MSWIN4.1");
        img[fat_offset + 11..fat_offset + 13].copy_from_slice(&512u16.to_le_bytes());
        img[fat_offset + 13] = 8; // sectors per cluster
        img[fat_offset + 21] = 0xF8; // media descriptor
        img[fat_offset + 510] = 0x55;
        img[fat_offset + 511] = 0xAA;
        std::fs::write(path, img).unwrap();
    }

    #[test]
    fn test_scan_mbr_image() {
        let path = "/tmp/testdisk_test.img";
        create_test_mbr_image(path);
        let result = scan_disk(path).unwrap();
        assert!(matches!(result.partition_table_type, PartitionTableType::Mbr));
        assert!(!result.partitions.is_empty());
        assert!(result.partitions[0].filesystem.is_some());
        let _ = std::fs::remove_file(path);
    }
}
