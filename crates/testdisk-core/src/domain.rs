use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub path: String,
    pub display_path: String,
    pub raw_path: String,
    pub platform_id: String,
    pub name: String,
    pub size_bytes: u64,
    pub readable: bool,
    pub writable: bool,
    pub source: DiskSource,
    pub kind: DiskKind,
    pub safety: TargetSafety,
    pub access: AccessCapability,
    pub protocol: Option<String>,
    pub is_internal: Option<bool>,
    pub is_removable: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiskSource {
    System,
    ImageFile,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiskKind {
    Physical,
    Image,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetSafety {
    Internal,
    External,
    Removable,
    Image,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessCapability {
    ReadWrite,
    ReadOnly,
    RequiresElevation,
    Unavailable,
}

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
    pub filesystem: Option<crate::fs_detect::FilesystemInfo>,
    pub status: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub disk_path: String,
    pub disk_size: u64,
    pub partition_table_type: PartitionTableType,
    pub mbr: Option<crate::mbr::MbrTable>,
    pub gpt: Option<crate::gpt::GptTable>,
    pub partitions: Vec<PartitionResult>,
    pub lost_partitions: Vec<PartitionResult>,
    pub warnings: Vec<String>,
}
