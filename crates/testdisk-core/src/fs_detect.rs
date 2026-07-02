use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub fs_type: String,
    pub confidence: String,
    pub label: Option<String>,
    pub details: String,
}

/// Detect filesystem from boot sector / superblock data (mirrors TestDisk analyse.c logic).
pub fn detect_filesystem(data: &[u8]) -> Option<FilesystemInfo> {
    if data.len() < 512 {
        return None;
    }

    // FAT / NTFS / exFAT — boot sector at offset 0, signature 0x55AA at 510
    if data.len() >= 512 {
        let sig = u16::from_le_bytes([data[510], data[511]]);
        if sig == 0xAA55 {
            if let Some(fs) = detect_fat(data) {
                return Some(fs);
            }
            if let Some(fs) = detect_ntfs(data) {
                return Some(fs);
            }
            if let Some(fs) = detect_exfat(data) {
                return Some(fs);
            }
        }
    }

    // ext2/3/4 superblock at offset 1024
    if data.len() >= 1088 {
        if let Some(fs) = detect_ext(data) {
            return Some(fs);
        }
    }

    // APFS at offset 0
    if data.len() >= 32 {
        if let Some(fs) = detect_apfs(data) {
            return Some(fs);
        }
    }

    // HFS+ at offset 1024
    if data.len() >= 1536 {
        if let Some(fs) = detect_hfs(data) {
            return Some(fs);
        }
    }

    None
}

fn detect_fat(data: &[u8]) -> Option<FilesystemInfo> {
    let media = data.get(21)?;
    if *media == 0xF8 || *media >= 0xF0 {
        let oem = String::from_utf8_lossy(&data[3..11]).trim().to_string();
        let sectors_per_cluster = data.get(13)?;
        if *sectors_per_cluster > 0 && *sectors_per_cluster <= 128 {
            let fat_type = if oem.starts_with("FAT32")
                || data.get(66).map(|b| *b & 0xF0 == 0x00).unwrap_or(false)
            {
                "FAT32"
            } else if oem.starts_with("FAT16") || oem.starts_with("MSWIN") {
                "FAT16"
            } else {
                "FAT12/16"
            };
            return Some(FilesystemInfo {
                fs_type: fat_type.to_string(),
                confidence: "high".to_string(),
                label: read_fat_label(data),
                details: format!("OEM: {oem}"),
            });
        }
    }
    None
}

fn read_fat_label(data: &[u8]) -> Option<String> {
    if data.len() >= 43 {
        let label_bytes = &data[43..54];
        let label = String::from_utf8_lossy(label_bytes).trim().to_string();
        if !label.is_empty() && label.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            return Some(label.trim().to_string());
        }
    }
    None
}

fn detect_ntfs(data: &[u8]) -> Option<FilesystemInfo> {
    let oem = &data[3..11];
    if oem.starts_with(b"NTFS") {
        let bps = u16::from_le_bytes([data[11], data[12]]);
        if bps == 512 || bps == 1024 || bps == 2048 || bps == 4096 {
            return Some(FilesystemInfo {
                fs_type: "NTFS".to_string(),
                confidence: "high".to_string(),
                label: None,
                details: format!("Bytes per sector: {bps}"),
            });
        }
    }
    None
}

fn detect_exfat(data: &[u8]) -> Option<FilesystemInfo> {
    if data.len() >= 12 && &data[3..11] == b"EXFAT   " {
        return Some(FilesystemInfo {
            fs_type: "exFAT".to_string(),
            confidence: "high".to_string(),
            label: None,
            details: "exFAT boot sector".to_string(),
        });
    }
    None
}

fn detect_ext(data: &[u8]) -> Option<FilesystemInfo> {
    let sb = &data[1024..];
    if sb.len() < 84 {
        return None;
    }
    let magic = u16::from_le_bytes([sb[56], sb[57]]);
    if magic == 0xEF53 {
        let rev = u32::from_le_bytes([sb[76], sb[77], sb[78], sb[79]]);
        let fs_type = match rev {
            0..=1 => "ext2",
            2..=3 => "ext3",
            _ => "ext4",
        };
        let blocks = u32::from_le_bytes([sb[4], sb[5], sb[6], sb[7]]);
        return Some(FilesystemInfo {
            fs_type: fs_type.to_string(),
            confidence: "high".to_string(),
            label: None,
            details: format!("Blocks: {blocks}, rev: {rev}"),
        });
    }
    None
}

fn detect_apfs(data: &[u8]) -> Option<FilesystemInfo> {
    let mut cursor = Cursor::new(data);
    let magic = cursor.read_u32::<LittleEndian>().ok()?;
    // NXSB = 0x4253584e
    if magic == 0x4253_584e {
        return Some(FilesystemInfo {
            fs_type: "APFS".to_string(),
            confidence: "high".to_string(),
            label: None,
            details: "Apple File System".to_string(),
        });
    }
    None
}

fn detect_hfs(data: &[u8]) -> Option<FilesystemInfo> {
    let sb = &data[1024..];
    if sb.len() < 4 {
        return None;
    }
    let sig = u16::from_be_bytes([sb[0], sb[1]]);
    // HFS = 0x4244, HFS+ = version 4 or 5
    if sig == 0x4244 {
        return Some(FilesystemInfo {
            fs_type: "HFS".to_string(),
            confidence: "high".to_string(),
            label: None,
            details: "Classic HFS".to_string(),
        });
    }
    if sb.len() >= 8 {
        let version = u16::from_be_bytes([sb[4], sb[5]]);
        if version == 4 || version == 5 {
            return Some(FilesystemInfo {
                fs_type: "HFS+".to_string(),
                confidence: "high".to_string(),
                label: None,
                details: format!("HFS+ version {version}"),
            });
        }
    }
    None
}
