use crate::domain::ScanResult;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCAN_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub schema_version: u32,
    pub generated_at_unix_seconds: u64,
    pub result: ScanResult,
}

pub fn build_scan_report(result: ScanResult) -> ScanReport {
    ScanReport {
        schema_version: SCAN_REPORT_SCHEMA_VERSION,
        generated_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        result,
    }
}

pub fn scan_report_json(result: ScanResult) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&build_scan_report(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PartitionTableType;

    #[test]
    fn exports_scan_report_with_stable_schema_version() {
        let result = ScanResult {
            disk_path: "/tmp/sample.img".to_string(),
            disk_size: 1024,
            partition_table_type: PartitionTableType::Unknown,
            mbr: None,
            gpt: None,
            partitions: vec![],
            lost_partitions: vec![],
            warnings: vec!["sample warning".to_string()],
        };

        let json = scan_report_json(result).unwrap();

        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"disk_path\": \"/tmp/sample.img\""));
        assert!(json.contains("\"sample warning\""));
    }
}
