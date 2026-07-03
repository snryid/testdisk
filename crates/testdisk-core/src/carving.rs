use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const CARVING_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum CarvingError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported family: {0}")]
    UnsupportedFamily(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarvingFamily {
    pub id: String,
    pub label: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarvedFileRecord {
    pub schema_version: u32,
    pub family_id: String,
    pub output_path: String,
    pub offset: u64,
    pub length: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarvingJobState {
    pub schema_version: u32,
    pub source_path: String,
    pub source_signature: String,
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarvingJobTranscript {
    pub schema_version: u32,
    pub source_path: String,
    pub output_dir: String,
    pub carved_files: Vec<CarvedFileRecord>,
    pub state: CarvingJobState,
    pub status: String,
}

pub fn builtin_carving_families() -> Vec<CarvingFamily> {
    vec![
        CarvingFamily {
            id: "jpeg".to_string(),
            label: "JPEG".to_string(),
            extensions: vec!["jpg".to_string(), "jpeg".to_string()],
        },
        CarvingFamily {
            id: "png".to_string(),
            label: "PNG".to_string(),
            extensions: vec!["png".to_string()],
        },
        CarvingFamily {
            id: "pdf".to_string(),
            label: "PDF".to_string(),
            extensions: vec!["pdf".to_string()],
        },
    ]
}

pub fn carve_files(
    source_path: &str,
    output_dir: &str,
    families: &[String],
) -> Result<CarvingJobTranscript, CarvingError> {
    let source = fs::read(source_path)?;
    let source_signature = format!("{}:{}", source_path, source.len());
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir)?;
    let mut carved_files = Vec::new();

    let wants = |family: &str| families.is_empty() || families.iter().any(|wanted| wanted == family);

    if wants("jpeg") {
        carve_by_marker(
            &source,
            source_path,
            output_dir,
            "jpeg",
            "jpg",
            &[0xFF, 0xD8, 0xFF],
            &[0xFF, 0xD9],
            &mut carved_files,
        )?;
    }
    if wants("png") {
        carve_png(&source, source_path, output_dir, &mut carved_files)?;
    }
    if wants("pdf") {
        carve_by_ascii_marker(
            &source,
            source_path,
            output_dir,
            "pdf",
            "pdf",
            b"%PDF-",
            b"%%EOF",
            &mut carved_files,
        )?;
    }

    let state = CarvingJobState {
        schema_version: CARVING_SCHEMA_VERSION,
        source_path: source_path.to_string(),
        source_signature,
        offset: source.len() as u64,
    };
    Ok(CarvingJobTranscript {
        schema_version: CARVING_SCHEMA_VERSION,
        source_path: source_path.to_string(),
        output_dir: output_dir.display().to_string(),
        carved_files,
        state,
        status: "completed".to_string(),
    })
}

fn carve_png(
    source: &[u8],
    source_path: &str,
    output_dir: &Path,
    carved_files: &mut Vec<CarvedFileRecord>,
) -> Result<(), CarvingError> {
    let signature = b"\x89PNG\r\n\x1a\n";
    let end_marker = b"IEND";
    let mut offset = 0usize;
    while let Some(start) = find_bytes(source, signature, offset) {
        if let Some(end) = find_bytes(source, end_marker, start + signature.len()) {
            let end = (end + 12).min(source.len());
            write_carved_file(
                source_path,
                output_dir,
                "png",
                start as u64,
                &source[start..end],
                carved_files,
            )?;
            offset = end;
        } else {
            break;
        }
    }
    Ok(())
}

fn carve_by_marker(
    source: &[u8],
    source_path: &str,
    output_dir: &Path,
    family_id: &str,
    _extension: &str,
    start_marker: &[u8],
    end_marker: &[u8],
    carved_files: &mut Vec<CarvedFileRecord>,
) -> Result<(), CarvingError> {
    let mut offset = 0usize;
    while let Some(start) = find_bytes(source, start_marker, offset) {
        if let Some(end) = find_bytes(source, end_marker, start + start_marker.len()) {
            let end = (end + end_marker.len()).min(source.len());
            write_carved_file(
                source_path,
                output_dir,
                family_id,
                start as u64,
                &source[start..end],
                carved_files,
            )?;
            offset = end;
        } else {
            break;
        }
    }
    Ok(())
}

fn carve_by_ascii_marker(
    source: &[u8],
    _source_path: &str,
    output_dir: &Path,
    family_id: &str,
    extension: &str,
    start_marker: &[u8],
    end_marker: &[u8],
    carved_files: &mut Vec<CarvedFileRecord>,
) -> Result<(), CarvingError> {
    carve_by_marker(
        source,
        _source_path,
        output_dir,
        family_id,
        extension,
        start_marker,
        end_marker,
        carved_files,
    )
}

fn write_carved_file(
    _source_path: &str,
    output_dir: &Path,
    family_id: &str,
    offset: u64,
    bytes: &[u8],
    carved_files: &mut Vec<CarvedFileRecord>,
) -> Result<(), CarvingError> {
    let extension = match family_id {
        "jpeg" => "jpg",
        "png" => "png",
        "pdf" => "pdf",
        _ => "bin",
    };
    let output_path = output_dir.join(format!("{family_id}-{offset:08x}.{extension}"));
    fs::write(&output_path, bytes)?;
    carved_files.push(CarvedFileRecord {
        schema_version: CARVING_SCHEMA_VERSION,
        family_id: family_id.to_string(),
        output_path: output_path.display().to_string(),
        offset,
        length: bytes.len() as u64,
        status: "carved".to_string(),
    });
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() || from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| index + from)
}

pub fn export_carving_json(transcript: &CarvingJobTranscript) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn carves_png_signature_fixture() {
        let source = std::env::temp_dir().join("testdisk-carve.bin");
        let output = std::env::temp_dir().join("testdisk-carve-out");
        let mut bytes = vec![0u8; 256];
        bytes[32..40].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[64..68].copy_from_slice(b"IEND");
        bytes[68..80].copy_from_slice(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0, 0, 0, 0]);
        fs::write(&source, bytes).unwrap();

        let transcript = carve_files(source.to_str().unwrap(), output.to_str().unwrap(), &["png".to_string()]).unwrap();
        assert_eq!(transcript.carved_files.len(), 1);

        let _ = fs::remove_file(source);
        let _ = fs::remove_dir_all(output);
    }
}
