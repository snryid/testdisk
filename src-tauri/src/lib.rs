use testdisk_core::{disk_info_from_image, list_disks, scan_disk, DiskInfo, ScanResult};

#[tauri::command]
fn get_disks() -> Vec<DiskInfo> {
    list_disks()
}

#[tauri::command]
fn scan_disk_path(path: String) -> Result<ScanResult, String> {
    scan_disk(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_image(path: String) -> Result<DiskInfo, String> {
    disk_info_from_image(&path).map_err(|e| e.to_string())
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
            open_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
