use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

pub const IMAGING_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum ImagingError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid state: {0}")]
    InvalidState(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImagingJobState {
    pub schema_version: u32,
    pub source_path: String,
    pub output_path: String,
    pub source_signature: String,
    pub bytes_copied: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImagingTriageSignal {
    pub smart_failure: bool,
    pub repeated_read_errors: bool,
    pub identity_changed: bool,
    pub throughput_drop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImagingRecommendation {
    pub recommended: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImagingTranscript {
    pub schema_version: u32,
    pub source_path: String,
    pub output_path: String,
    pub bytes_copied: u64,
    pub checksum: String,
    pub read_errors: u64,
    pub throughput_bytes_per_sec: u64,
    pub state: ImagingJobState,
    pub status: String,
}

pub fn image_source(
    source_path: &str,
    output_path: &str,
) -> Result<ImagingTranscript, ImagingError> {
    let mut source = File::open(source_path)?;
    let mut output = File::create(output_path)?;
    let mut buffer = [0u8; 128 * 1024];
    let mut hasher = Hasher::new();
    let mut copied = 0u64;
    let mut read_errors = 0u64;
    let started = std::time::Instant::now();

    loop {
        let read = match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(_) => {
                read_errors += 1;
                break;
            }
        };
        output.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        copied += read as u64;
    }

    let checksum = format!("{:08x}", hasher.finalize());
    let elapsed = started.elapsed().as_secs().max(1);
    let throughput = copied / elapsed;
    let state = ImagingJobState {
        schema_version: IMAGING_SCHEMA_VERSION,
        source_path: source_path.to_string(),
        output_path: output_path.to_string(),
        source_signature: format!("{}:{}", source_path, copied),
        bytes_copied: copied,
        checksum: checksum.clone(),
    };

    Ok(ImagingTranscript {
        schema_version: IMAGING_SCHEMA_VERSION,
        source_path: source_path.to_string(),
        output_path: output_path.to_string(),
        bytes_copied: copied,
        checksum,
        read_errors,
        throughput_bytes_per_sec: throughput,
        state,
        status: "completed".to_string(),
    })
}

pub fn write_imaging_state(path: &Path, state: &ImagingJobState) -> Result<(), ImagingError> {
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn read_imaging_state(path: &Path) -> Result<ImagingJobState, ImagingError> {
    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

pub fn triage_from_signals(signals: &ImagingTriageSignal) -> ImagingRecommendation {
    if signals.smart_failure || signals.repeated_read_errors || signals.identity_changed {
        ImagingRecommendation {
            recommended: true,
            reason: "image-first workflow is recommended".to_string(),
        }
    } else if signals.throughput_drop {
        ImagingRecommendation {
            recommended: true,
            reason: "scan speed drop suggests imaging before repair".to_string(),
        }
    } else {
        ImagingRecommendation {
            recommended: false,
            reason: "risk signals are not currently elevated".to_string(),
        }
    }
}

pub fn export_imaging_json(transcript: &ImagingTranscript) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn images_fixture_and_records_checksum() {
        let source = std::env::temp_dir().join("testdisk-image-source.bin");
        let output = std::env::temp_dir().join("testdisk-image-output.bin");
        fs::write(&source, b"imaging fixture").unwrap();

        let transcript = image_source(source.to_str().unwrap(), output.to_str().unwrap()).unwrap();
        assert_eq!(transcript.bytes_copied, 15);
        assert!(!transcript.checksum.is_empty());

        let _ = fs::remove_file(source);
        let _ = fs::remove_file(output);
    }
}
