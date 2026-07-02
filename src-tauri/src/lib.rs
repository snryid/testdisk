use testdisk_core::{scan_disk, scan_report_json, ScanResult};
use testdisk_platform::{
    format_disk, format_filesystem_options, host_platform_adapter, DiskInfo, FormatDiskRequest,
    FormatDiskResult, FormatFilesystemOption, PlatformAdapter,
};

#[tauri::command]
fn get_disks() -> Vec<DiskInfo> {
    host_platform_adapter().list_disks()
}

#[tauri::command]
fn scan_disk_path(path: String) -> Result<ScanResult, String> {
    scan_disk(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_image(path: String) -> Result<DiskInfo, String> {
    host_platform_adapter()
        .open_image(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_format_filesystems(target: Option<DiskInfo>) -> Vec<FormatFilesystemOption> {
    format_filesystem_options(target.as_ref())
}

#[tauri::command]
fn format_disk_path(request: FormatDiskRequest) -> Result<FormatDiskResult, String> {
    let target = trusted_format_target(&request.path, &request.target)?;
    let request = FormatDiskRequest { target, ..request };
    format_disk(request).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_scan_report_json(path: String, result: ScanResult) -> Result<(), String> {
    let json = scan_report_json(result).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn trusted_format_target(path: &str, target: &DiskInfo) -> Result<DiskInfo, String> {
    let current = host_platform_adapter()
        .list_disks()
        .into_iter()
        .find(|disk| disk.path == path || disk.platform_id == target.platform_id)
        .ok_or_else(|| format!("format target not found: {path}"))?;

    if current.path != target.path || current.platform_id != target.platform_id {
        return Err("format target no longer matches current disk metadata".to_string());
    }

    Ok(current)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_disks,
            scan_disk_path,
            open_image,
            get_format_filesystems,
            format_disk_path,
            export_scan_report_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
