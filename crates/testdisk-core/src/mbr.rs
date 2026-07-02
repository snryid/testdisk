use crate::disk::{DiskReader, SECTOR_SIZE};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Cursor;

const EXTENDED_TYPES: [u8; 4] = [0x05, 0x0f, 0x85, 0x91];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbrPartition {
    pub index: u8,
    pub status: u8,
    pub partition_type: u8,
    pub start_lba: u64,
    pub sector_count: u64,
    pub size_bytes: u64,
    pub type_name: String,
    pub bootable: bool,
    pub logical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbrTable {
    pub partitions: Vec<MbrPartition>,
    pub signature_valid: bool,
}

pub fn mbr_type_name(type_code: u8) -> &'static str {
    match type_code {
        0x00 => "Empty",
        0x01 => "FAT12",
        0x04 => "FAT16 (<32M)",
        0x05 => "Extended",
        0x06 => "FAT16",
        0x07 => "NTFS/exFAT",
        0x0b => "FAT32",
        0x0c => "FAT32 LBA",
        0x0e => "FAT16 LBA",
        0x0f => "Extended LBA",
        0x82 => "Linux Swap",
        0x83 => "Linux",
        0x8e => "Linux LVM",
        0xee => "GPT Protective",
        0xef => "EFI System",
        _ => "Unknown",
    }
}

pub fn read_mbr(disk: &mut DiskReader) -> Result<MbrTable, crate::disk::DiskError> {
    let sector = disk.read_sector(0)?;
    let mut partitions = Vec::new();
    let mut extended_base_lba = None;

    for i in 0..4 {
        let Some(entry) = read_partition_entry(&sector, i) else {
            continue;
        };
        if entry.partition_type != 0 {
            if is_extended_type(entry.partition_type) {
                extended_base_lba = Some(entry.start_lba);
            }
            partitions.push(MbrPartition {
                index: (i + 1) as u8,
                status: entry.status,
                partition_type: entry.partition_type,
                start_lba: entry.start_lba,
                sector_count: entry.sector_count,
                size_bytes: entry.sector_count * SECTOR_SIZE,
                type_name: mbr_type_name(entry.partition_type).to_string(),
                bootable: entry.status == 0x80,
                logical: false,
            });
        }
    }

    if let Some(base_lba) = extended_base_lba {
        let mut current_ebr_lba = base_lba;
        let mut logical_index = 5u8;
        let mut visited = HashSet::new();

        while visited.insert(current_ebr_lba) && current_ebr_lba > 0 {
            let ebr_sector = disk.read_sector(current_ebr_lba)?;
            let Some(logical_entry) = read_partition_entry(&ebr_sector, 0) else {
                break;
            };

            if logical_entry.partition_type != 0 {
                let start_lba = current_ebr_lba + logical_entry.start_lba;
                partitions.push(MbrPartition {
                    index: logical_index,
                    status: logical_entry.status,
                    partition_type: logical_entry.partition_type,
                    start_lba,
                    sector_count: logical_entry.sector_count,
                    size_bytes: logical_entry.sector_count * SECTOR_SIZE,
                    type_name: mbr_type_name(logical_entry.partition_type).to_string(),
                    bootable: logical_entry.status == 0x80,
                    logical: true,
                });
                logical_index = logical_index.saturating_add(1);
            }

            let Some(next_entry) = read_partition_entry(&ebr_sector, 1) else {
                break;
            };
            if !is_extended_type(next_entry.partition_type) || next_entry.start_lba == 0 {
                break;
            }
            current_ebr_lba = base_lba + next_entry.start_lba;
        }
    }

    let mut cursor = Cursor::new(&sector);
    cursor.set_position(510);
    let sig = cursor
        .read_u16::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    Ok(MbrTable {
        partitions,
        signature_valid: sig == 0xAA55,
    })
}

pub fn is_gpt_protective_mbr(mbr: &MbrTable) -> bool {
    mbr.partitions.iter().any(|p| p.partition_type == 0xee)
}

fn is_extended_type(partition_type: u8) -> bool {
    EXTENDED_TYPES.contains(&partition_type)
}

struct RawMbrEntry {
    status: u8,
    partition_type: u8,
    start_lba: u64,
    sector_count: u64,
}

fn read_partition_entry(sector: &[u8], entry_index: usize) -> Option<RawMbrEntry> {
    let offset = 446 + entry_index * 16;
    let entry = sector.get(offset..offset + 16)?;
    let mut cursor = Cursor::new(entry);
    let status = cursor.read_u8().ok()?;
    cursor.set_position(cursor.position() + 3);
    let partition_type = cursor.read_u8().ok()?;
    cursor.set_position(cursor.position() + 3);
    let start_lba = cursor.read_u32::<LittleEndian>().ok()? as u64;
    let sector_count = cursor.read_u32::<LittleEndian>().ok()? as u64;
    Some(RawMbrEntry {
        status,
        partition_type,
        start_lba,
        sector_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_u32_le(img: &mut [u8], offset: usize, value: u32) {
        img[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn create_extended_chain_image(path: &PathBuf) {
        let mut img = vec![0u8; 8 * 1024 * 1024];
        img[510] = 0x55;
        img[511] = 0xAA;

        let primary_offset = 446;
        img[primary_offset + 4] = 0x05;
        write_u32_le(&mut img, primary_offset + 8, 2048);
        write_u32_le(&mut img, primary_offset + 12, 4096);

        let ebr_offset = 2048 * 512;
        img[ebr_offset + 510] = 0x55;
        img[ebr_offset + 511] = 0xAA;

        let logical_offset = ebr_offset + 446;
        img[logical_offset + 4] = 0x0b;
        write_u32_le(&mut img, logical_offset + 8, 2048);
        write_u32_le(&mut img, logical_offset + 12, 1024);

        std::fs::write(path, img).unwrap();
    }

    #[test]
    fn reads_logical_partitions_from_extended_chain() {
        let temp_path: PathBuf = std::env::temp_dir().join("testdisk-mbr-extended.img");
        create_extended_chain_image(&temp_path);

        let mut disk = DiskReader::open(&temp_path).unwrap();
        let table = read_mbr(&mut disk).unwrap();

        assert!(table.partitions.iter().any(|partition| partition.partition_type == 0x05));
        assert!(table.partitions.iter().any(|partition| partition.logical));

        let _ = std::fs::remove_file(temp_path);
    }
}
