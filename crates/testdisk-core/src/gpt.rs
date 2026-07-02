use crate::disk::{DiskReader, SECTOR_SIZE};
use byteorder::{LittleEndian, ReadBytesExt};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptPartition {
    pub index: u32,
    pub type_guid: String,
    pub partition_guid: String,
    pub name: String,
    pub start_lba: u64,
    pub end_lba: u64,
    pub size_bytes: u64,
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptTable {
    pub header_lba: u64,
    pub partitions: Vec<GptPartition>,
    pub disk_guid: String,
    pub signature_valid: bool,
    pub crc_valid: bool,
}

pub fn gpt_type_name(guid: &str) -> &'static str {
    match guid.to_uppercase().as_str() {
        "C12A7328-F81F-11D2-BA4B-00A0C93EC93B" => "EFI System",
        "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7" => "Microsoft Basic Data",
        "E3C9E316-0B5C-4DB8-817D-F92DF00215AE" => "Microsoft Reserved",
        "DE94BBA4-06D1-4D40-A16A-BFD50179D6AC" => "Windows Recovery",
        "0FC63DAF-8483-4772-8E79-3D69D8477DE4" => "Linux filesystem",
        "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F" => "Linux Swap",
        "E6D6D379-F507-44C2-805E-16358CC8F43E" => "Linux LVM",
        "48465300-0000-11AA-AA11-00306543ECAC" => "Apple HFS+",
        "7C3457EF-0000-11AA-AA11-00306543ECAC" => "Apple APFS",
        "516E7CB4-6ECF-11D6-8FF8-000393217D9E" => "Apple Boot",
        _ => "Unknown",
    }
}

fn format_guid(bytes: &[u8]) -> String {
    if bytes.len() < 16 {
        return String::new();
    }
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_le_bytes([bytes[4], bytes[5]]),
        u16::from_le_bytes([bytes[6], bytes[7]]),
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

fn read_utf16_name(raw: &[u8]) -> String {
    let chars: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf16_lossy(&chars)
}

pub fn read_gpt(
    disk: &mut DiskReader,
    header_lba: u64,
) -> Result<GptTable, crate::disk::DiskError> {
    let header_sector = disk.read_sector(header_lba)?;
    let mut cursor = Cursor::new(&header_sector);

    let mut sig = [0u8; 8];
    cursor.read_exact(&mut sig).map_err(std::io::Error::other)?;
    if &sig != GPT_SIGNATURE {
        return Ok(GptTable {
            header_lba,
            partitions: vec![],
            disk_guid: String::new(),
            signature_valid: false,
            crc_valid: false,
        });
    }

    let _revision = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let header_size = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)? as usize;
    let crc_stored = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let _reserved = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let _lba_self = cursor
        .read_u64::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let _lba_alt = cursor
        .read_u64::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let _lba_start = cursor
        .read_u64::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let _lba_end = cursor
        .read_u64::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let lba_table = cursor
        .read_u64::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let entry_count = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)?;
    let entry_size = cursor
        .read_u32::<LittleEndian>()
        .map_err(std::io::Error::other)?;

    let mut disk_guid_bytes = [0u8; 16];
    cursor
        .read_exact(&mut disk_guid_bytes)
        .map_err(std::io::Error::other)?;
    let disk_guid = format_guid(&disk_guid_bytes);

    let mut header_for_crc = header_sector.clone();
    header_for_crc[16..20].copy_from_slice(&0u32.to_le_bytes());
    let mut hasher = Hasher::new();
    hasher.update(&header_for_crc[..header_size.min(header_sector.len())]);
    let crc_calc = hasher.finalize();
    let crc_valid = crc_calc == crc_stored;

    let entries_size = (entry_count as u64) * (entry_size as u64);
    let entries_data = disk.read_at(lba_table * SECTOR_SIZE, entries_size as usize)?;

    let mut partitions = Vec::new();
    for i in 0..entry_count {
        let offset = (i as usize) * (entry_size as usize);
        if offset + 128 > entries_data.len() {
            break;
        }
        let entry = &entries_data[offset..offset + entry_size as usize];

        if entry[0..16].iter().all(|&b| b == 0) {
            continue;
        }

        let type_guid = format_guid(&entry[0..16]);
        let part_guid = format_guid(&entry[16..32]);
        let start_lba = u64::from_le_bytes(entry[32..40].try_into().unwrap());
        let end_lba = u64::from_le_bytes(entry[40..48].try_into().unwrap());
        let name = read_utf16_name(&entry[56..128.min(entry.len())]);

        if start_lba > 0 && end_lba >= start_lba {
            partitions.push(GptPartition {
                index: i + 1,
                type_guid: type_guid.clone(),
                partition_guid: part_guid,
                name,
                start_lba,
                end_lba,
                size_bytes: (end_lba - start_lba + 1) * SECTOR_SIZE,
                type_name: gpt_type_name(&type_guid).to_string(),
            });
        }
    }

    Ok(GptTable {
        header_lba,
        partitions,
        disk_guid,
        signature_valid: true,
        crc_valid,
    })
}

pub fn detect_gpt(disk: &mut DiskReader) -> Result<Option<GptTable>, crate::disk::DiskError> {
    let primary = read_gpt(disk, 1)?;
    if primary.signature_valid {
        return Ok(Some(primary));
    }

    let last_lba = (disk.size / SECTOR_SIZE).saturating_sub(1);
    let backup = read_gpt(disk, last_lba)?;
    if backup.signature_valid {
        return Ok(Some(backup));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::DiskReader;
    use std::path::PathBuf;

    fn write_u32_le(img: &mut [u8], offset: usize, value: u32) {
        img[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64_le(img: &mut [u8], offset: usize, value: u64) {
        img[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn create_backup_gpt_image(path: &PathBuf) {
        let sectors = 8192usize;
        let mut img = vec![0u8; sectors * 512];
        let last_lba = (sectors as u64) - 1;
        let entries_lba = last_lba - 1;

        let header_offset = (last_lba as usize) * 512;
        img[header_offset..header_offset + 8].copy_from_slice(GPT_SIGNATURE);
        write_u32_le(&mut img, header_offset + 8, 0x0001_0000);
        write_u32_le(&mut img, header_offset + 12, 92);
        write_u32_le(&mut img, header_offset + 16, 0);
        write_u64_le(&mut img, header_offset + 24, last_lba);
        write_u64_le(&mut img, header_offset + 32, 1);
        write_u64_le(&mut img, header_offset + 40, 34);
        write_u64_le(&mut img, header_offset + 48, last_lba - 34);
        write_u64_le(&mut img, header_offset + 56, entries_lba);
        write_u32_le(&mut img, header_offset + 64, 1);
        write_u32_le(&mut img, header_offset + 68, 128);
        img[header_offset + 72..header_offset + 88].fill(0x11);

        let entry_offset = (entries_lba as usize) * 512;
        img[entry_offset..entry_offset + 16].fill(0x22);
        img[entry_offset + 16..entry_offset + 32].fill(0x33);
        write_u64_le(&mut img, entry_offset + 32, 2048);
        write_u64_le(&mut img, entry_offset + 40, 4095);
        let name = "Backup GPT".encode_utf16().collect::<Vec<u16>>();
        for (i, unit) in name.iter().enumerate() {
            let offset = entry_offset + 56 + i * 2;
            img[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }

        std::fs::write(path, img).unwrap();
    }

    #[test]
    fn detects_backup_gpt_header() {
        let temp_path: PathBuf = std::env::temp_dir().join("testdisk-gpt-backup.img");
        create_backup_gpt_image(&temp_path);

        let mut disk = DiskReader::open(&temp_path).unwrap();
        let table = detect_gpt(&mut disk).unwrap().unwrap();

        assert_eq!(table.header_lba, 8191);
        assert!(table.signature_valid);
        assert_eq!(table.partitions.len(), 1);

        let _ = std::fs::remove_file(temp_path);
    }
}
