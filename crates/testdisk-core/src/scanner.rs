use crate::disk::{DiskError, DiskReader, SECTOR_SIZE};
use crate::domain::{PartitionResult, PartitionTableType, ScanResult};
use crate::fs_detect::{detect_filesystem, FilesystemInfo};
use crate::gpt::{detect_gpt, GptTable};
use crate::mbr::{is_gpt_protective_mbr, read_mbr, MbrTable};
use std::collections::HashSet;

const DEEP_SCAN_STEPS: [u64; 2] = [128, 2048];

/// Main scan mirrors TestDisk's autodetect_arch + read_part flow.
pub fn scan_disk(path: &str) -> Result<ScanResult, DiskError> {
    let mut disk = DiskReader::open(path)?;
    let disk_size = disk.size;
    let mut warnings = Vec::new();

    let mbr = read_mbr(&mut disk).ok();
    let gpt = detect_gpt(&mut disk)?;

    let (table_type, mut partitions) = if let Some(ref gpt_table) = gpt {
        if !gpt_table.crc_valid {
            warnings.push("GPT 头 CRC 校验失败，分区表可能已损坏".to_string());
        }
        if gpt_table.header_lba != 1 {
            warnings.push("检测到 GPT 备份头，主头不可用".to_string());
        }
        (
            PartitionTableType::Gpt,
            analyze_gpt_partitions(&mut disk, gpt_table),
        )
    } else if let Some(ref mbr_table) = mbr {
        if !mbr_table.signature_valid {
            warnings.push("MBR 签名无效 (0x55AA)".to_string());
        }
        if is_gpt_protective_mbr(mbr_table) {
            warnings.push("检测到 GPT 保护性 MBR，但 GPT 头未找到".to_string());
        }
        (
            PartitionTableType::Mbr,
            analyze_mbr_partitions(&mut disk, mbr_table),
        )
    } else {
        warnings.push("未检测到有效的分区表".to_string());
        (PartitionTableType::Unknown, vec![])
    };

    if let (Some(gpt_table), Some(mbr_table)) = (&gpt, &mbr) {
        if is_hybrid_mbr(mbr_table, gpt_table) {
            warnings.push("检测到混合 MBR：保护性 GPT 之外还有额外 MBR 分区".to_string());
        }
    }

    annotate_partition_conflicts(&mut partitions, disk_size, &mut warnings);

    let mut lost_partitions = search_lost_partitions(&mut disk, &partitions, disk_size);
    annotate_partition_conflicts(&mut lost_partitions, disk_size, &mut warnings);

    Ok(ScanResult {
        disk_path: disk.path.clone(),
        disk_size,
        partition_table_type: table_type,
        mbr,
        gpt,
        partitions,
        lost_partitions,
        warnings,
    })
}

fn analyze_gpt_partitions(disk: &mut DiskReader, gpt: &GptTable) -> Vec<PartitionResult> {
    gpt.partitions
        .iter()
        .map(|partition| {
            build_partition_result(
                partition.index,
                if partition.name.is_empty() {
                    partition.type_name.clone()
                } else {
                    partition.name.clone()
                },
                partition.start_lba,
                partition.end_lba,
                partition.size_bytes,
                partition.type_name.clone(),
                detect_fs_at(disk, partition.start_lba * SECTOR_SIZE),
                "primary".to_string(),
                "gpt".to_string(),
                0.98,
                "clear".to_string(),
            )
        })
        .collect()
}

fn analyze_mbr_partitions(disk: &mut DiskReader, mbr: &MbrTable) -> Vec<PartitionResult> {
    mbr.partitions
        .iter()
        .map(|partition| {
            let status = if partition.bootable {
                "bootable".to_string()
            } else {
                "primary".to_string()
            };
            build_partition_result(
                partition.index as u32,
                if partition.logical {
                    format!("Logical {}", partition.index)
                } else {
                    format!("Partition {}", partition.index)
                },
                partition.start_lba,
                partition
                    .start_lba
                    .saturating_add(partition.sector_count.saturating_sub(1)),
                partition.sector_count * SECTOR_SIZE,
                partition.type_name.clone(),
                detect_fs_at(disk, partition.start_lba * SECTOR_SIZE),
                status,
                if partition.logical {
                    "mbr-logical".to_string()
                } else {
                    "mbr".to_string()
                },
                if partition.logical { 0.94 } else { 0.96 },
                "clear".to_string(),
            )
        })
        .collect()
}

fn detect_fs_at(disk: &mut DiskReader, offset: u64) -> Option<FilesystemInfo> {
    let data = disk.read_at(offset, 8192).ok()?;
    detect_filesystem(&data)
}

fn search_lost_partitions(
    disk: &mut DiskReader,
    known: &[PartitionResult],
    disk_size: u64,
) -> Vec<PartitionResult> {
    let mut found = Vec::new();
    let mut seen_offsets = HashSet::new();
    let probe_limit = disk_size.saturating_sub(8192);

    for step in DEEP_SCAN_STEPS {
        let mut lba = step;
        while lba.saturating_mul(SECTOR_SIZE) < probe_limit {
            let offset = lba * SECTOR_SIZE;
            if !seen_offsets.insert(offset) {
                lba += step;
                continue;
            }
            if is_covered_by_known(offset, known) {
                lba += step;
                continue;
            }

            if let Ok(data) = disk.read_at(offset, 8192) {
                if let Some(fs) = detect_filesystem(&data) {
                    let size_estimate = estimate_fs_size(&data, fs.fs_type.as_str());
                    let size_bytes = size_estimate.min(disk_size.saturating_sub(offset));
                    let start_lba = offset / SECTOR_SIZE;
                    let end_lba =
                        start_lba + size_bytes.saturating_div(SECTOR_SIZE).saturating_sub(1);
                    let fs_confidence = fs.confidence.clone();
                    found.push(build_partition_result(
                        (found.len() + 1) as u32,
                        format!("Lost: {}", fs.fs_type),
                        start_lba,
                        end_lba,
                        size_bytes,
                        "Lost/Deleted".to_string(),
                        Some(fs),
                        "deleted".to_string(),
                        "deep_scan".to_string(),
                        confidence_from_filesystem(fs_confidence.as_str(), step),
                        "clear".to_string(),
                    ));
                }
            }

            lba += step;
        }
    }

    found
}

fn confidence_from_filesystem(fs_confidence: &str, step: u64) -> f32 {
    let mut base: f32 = match fs_confidence {
        "high" => 0.9,
        "medium" => 0.7,
        "low" => 0.55,
        _ => 0.6,
    };
    if step <= 128 {
        base += 0.05;
    } else if step >= 2048 {
        base -= 0.05;
    }
    base.clamp(0.0, 1.0)
}

fn build_partition_result(
    index: u32,
    name: String,
    start_lba: u64,
    end_lba: u64,
    size_bytes: u64,
    type_name: String,
    filesystem: Option<FilesystemInfo>,
    status: String,
    source: String,
    confidence: f32,
    conflict_status: String,
) -> PartitionResult {
    PartitionResult {
        index,
        name,
        start_lba,
        end_lba,
        size_bytes,
        type_name,
        filesystem,
        status,
        source,
        confidence,
        conflict_status,
    }
}

fn is_covered_by_known(offset: u64, known: &[PartitionResult]) -> bool {
    known.iter().any(|p| {
        let start = p.start_lba * SECTOR_SIZE;
        let end = (p.end_lba + 1) * SECTOR_SIZE;
        offset >= start && offset < end
    })
}

fn annotate_partition_conflicts(
    partitions: &mut [PartitionResult],
    disk_size: u64,
    warnings: &mut Vec<String>,
) {
    let max_lba = disk_size / SECTOR_SIZE;

    for i in 0..partitions.len() {
        let mut conflict_status = if partitions[i].end_lba >= max_lba {
            "out_of_bounds".to_string()
        } else {
            "clear".to_string()
        };

        for j in 0..partitions.len() {
            if i == j {
                continue;
            }
            if ranges_overlap(
                partitions[i].start_lba,
                partitions[i].end_lba,
                partitions[j].start_lba,
                partitions[j].end_lba,
            ) {
                conflict_status = "overlap".to_string();
                break;
            }
        }

        if conflict_status != "clear" {
            warnings.push(format!(
                "分区 {} ({}) 状态冲突: {}",
                partitions[i].index, partitions[i].name, conflict_status
            ));
            partitions[i].confidence = (partitions[i].confidence - 0.15).clamp(0.0, 1.0);
        }
        partitions[i].conflict_status = conflict_status;
    }
}

fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start <= b_end && b_start <= a_end
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
            100 * 1024 * 1024
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
        "APFS" | "HFS+" | "HFS" => 1024 * 1024 * 1024,
        _ => 50 * 1024 * 1024,
    }
}

fn is_hybrid_mbr(mbr: &MbrTable, gpt: &GptTable) -> bool {
    if !gpt.signature_valid {
        return false;
    }
    let non_empty_mbr_partitions = mbr
        .partitions
        .iter()
        .filter(|partition| partition.partition_type != 0 && partition.partition_type != 0xee)
        .count();
    non_empty_mbr_partitions > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(path: &str, data: Vec<u8>) {
        fs::write(path, data).unwrap();
    }

    #[test]
    fn test_scan_mbr_image() {
        let path = "/tmp/testdisk_test.img";
        let mut img = vec![0u8; 4 * 1024 * 1024];
        img[510] = 0x55;
        img[511] = 0xAA;
        let entry_offset = 446;
        img[entry_offset + 4] = 0x0c;
        img[entry_offset + 8..entry_offset + 12].copy_from_slice(&2048u32.to_le_bytes());
        img[entry_offset + 12..entry_offset + 16].copy_from_slice(&2048u32.to_le_bytes());
        let fat_offset = 2048 * 512;
        img[fat_offset + 3..fat_offset + 11].copy_from_slice(b"MSWIN4.1");
        img[fat_offset + 11..fat_offset + 13].copy_from_slice(&512u16.to_le_bytes());
        img[fat_offset + 13] = 8;
        img[fat_offset + 21] = 0xF8;
        img[fat_offset + 510] = 0x55;
        img[fat_offset + 511] = 0xAA;
        write_file(path, img);

        let result = scan_disk(path).unwrap();
        assert!(matches!(
            result.partition_table_type,
            PartitionTableType::Mbr
        ));
        assert!(!result.partitions.is_empty());
        assert!(result.partitions[0].filesystem.is_some());
        assert!((result.partitions[0].confidence - 0.96).abs() < f32::EPSILON);
        let _ = fs::remove_file(path);
    }
}
