use crate::disk::{DiskReader, SECTOR_SIZE};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

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
    let mut cursor = Cursor::new(&sector);

    cursor.set_position(446);

    let mut partitions = Vec::new();
    for i in 0..4 {
        let status = cursor.read_u8().map_err(std::io::Error::other)?;
        cursor.set_position(cursor.position() + 3);
        let part_type = cursor.read_u8().map_err(std::io::Error::other)?;
        cursor.set_position(cursor.position() + 3);
        let start_lba = cursor
            .read_u32::<LittleEndian>()
            .map_err(std::io::Error::other)? as u64;
        let sector_count = cursor
            .read_u32::<LittleEndian>()
            .map_err(std::io::Error::other)? as u64;

        if part_type != 0 {
            partitions.push(MbrPartition {
                index: i + 1,
                status,
                partition_type: part_type,
                start_lba,
                sector_count,
                size_bytes: sector_count * SECTOR_SIZE,
                type_name: mbr_type_name(part_type).to_string(),
                bootable: status == 0x80,
            });
        }
    }

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
