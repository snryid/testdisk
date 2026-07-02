use crate::disk::{disk_info_from_image, list_disks, DiskError};
use crate::formatter::{
    format_disk, format_filesystem_options, FormatDiskRequest, FormatDiskResult, FormatError,
    FormatFilesystemOption,
};
use testdisk_core::DiskInfo;

pub trait PlatformAdapter {
    fn list_disks(&self) -> Vec<DiskInfo>;
    fn open_image(&self, path: &str) -> Result<DiskInfo, DiskError>;
    fn format_filesystems(&self) -> Vec<FormatFilesystemOption>;
    fn format_disk(&self, request: FormatDiskRequest) -> Result<FormatDiskResult, FormatError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HostPlatformAdapter;

impl PlatformAdapter for HostPlatformAdapter {
    fn list_disks(&self) -> Vec<DiskInfo> {
        list_disks()
    }

    fn open_image(&self, path: &str) -> Result<DiskInfo, DiskError> {
        disk_info_from_image(path)
    }

    fn format_filesystems(&self) -> Vec<FormatFilesystemOption> {
        format_filesystem_options()
    }

    fn format_disk(&self, request: FormatDiskRequest) -> Result<FormatDiskResult, FormatError> {
        format_disk(request)
    }
}

pub fn host_platform_adapter() -> HostPlatformAdapter {
    HostPlatformAdapter
}
