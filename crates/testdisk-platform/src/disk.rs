pub use testdisk_core::{DiskError, DiskInfo, DiskReader, SECTOR_SIZE};

pub fn list_disks() -> Vec<DiskInfo> {
    testdisk_core::list_disks()
}

pub fn disk_info_from_image(path: &str) -> Result<DiskInfo, DiskError> {
    testdisk_core::disk_info_from_image(path)
}
