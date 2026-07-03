use testdisk_core::{
    carve_files, diagnose_boot_sectors as run_diagnose_boot_sectors, image_source,
    preview_boot_sector_repair, preview_format_plan as build_preview_format_plan,
    recover_fat_deleted_files, scan_disk, scan_report_json, CarvingJobTranscript,
    BootSectorDiagnosis, BootSectorRepairPlan, ImagingTranscript, RecoveryJobTranscript, ScanResult,
};
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

#[tauri::command]
fn preview_format_plan_command(request: FormatDiskRequest) -> Result<testdisk_core::FormatOperationPlan, String> {
    build_preview_format_plan(&request).map_err(|e| e.to_string())
}

#[tauri::command]
fn diagnose_boot_sectors_command(path: String) -> Result<Vec<BootSectorDiagnosis>, String> {
    run_diagnose_boot_sectors(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_boot_sector_repair_command(path: String) -> Result<Vec<BootSectorRepairPlan>, String> {
    preview_boot_sector_repair(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn recover_deleted_files(source_path: String, destination_dir: String) -> Result<RecoveryJobTranscript, String> {
    recover_fat_deleted_files(&source_path, &destination_dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn carve_files_command(
    source_path: String,
    output_dir: String,
    families: Vec<String>,
) -> Result<CarvingJobTranscript, String> {
    carve_files(&source_path, &output_dir, &families).map_err(|e| e.to_string())
}

#[tauri::command]
fn image_source_command(source_path: String, output_path: String) -> Result<ImagingTranscript, String> {
    image_source(&source_path, &output_path).map_err(|e| e.to_string())
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
            export_scan_report_json,
            preview_format_plan_command,
            diagnose_boot_sectors_command,
            preview_boot_sector_repair_command,
            recover_deleted_files,
            carve_files_command,
            image_source_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
