use serde::Serialize;
use std::fmt;
use std::fs;
use testdisk_core::{
    import_recovery_plan_json, preview_format_plan, scan_disk, scan_report_json,
    FormatDiskRequest, FormatFilesystem, RecoveryPlanValidationSeverity,
};
use testdisk_platform::{
    format_disk, format_filesystem_options, host_platform_adapter, PlatformAdapter,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CliEnvelope<T> {
    pub ok: bool,
    pub command: String,
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    NotImplemented(&'static str),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::NotImplemented(command) => write!(f, "{command} is not implemented"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn dispatch<I, S>(args: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into).collect::<Vec<String>>();
    if args.is_empty() {
        return Err(CliError::NotImplemented("dispatch"));
    }
    if args[0].ends_with("mini-testdisk") || args[0] == "mini-testdisk" {
        args.remove(0);
    }
    match args.first().map(String::as_str) {
        Some("list-filesystems") => list_filesystems_json().map_err(|_| CliError::NotImplemented("dispatch")),
        Some("scan") => run_scan_command(&args[1..]).map_err(|_| CliError::NotImplemented("scan")),
        Some("validate-plan") => {
            run_validate_plan_command(&args[1..]).map_err(|_| CliError::NotImplemented("validate-plan"))
        }
        Some("format") => run_format_command(&args[1..]).map_err(|_| CliError::NotImplemented("format")),
        _ => Err(CliError::NotImplemented("dispatch")),
    }
}

pub fn list_filesystems_json() -> Result<String, serde_json::Error> {
    let options = format_filesystem_options(None);
    envelope_json("list-filesystems", "ok", "supported filesystems", Some(options))
}

pub fn scan_report_json_from_path(path: &str) -> Result<String, String> {
    scan_disk(path)
        .map_err(|error| error.to_string())
        .and_then(|result| scan_report_json(result).map_err(|error| error.to_string()))
}

pub fn preview_format_json(request: &FormatDiskRequest) -> Result<String, serde_json::Error> {
    let plan = preview_format_plan(request).map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    envelope_json("format", "ok", "preview-only format plan", Some(plan))
}

pub fn resolve_disks() -> Vec<testdisk_core::DiskInfo> {
    host_platform_adapter().list_disks()
}

fn envelope_json<T>(
    command: &str,
    code: &str,
    message: &str,
    data: Option<T>,
) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    serde_json::to_string_pretty(&CliEnvelope {
        ok: code == "ok",
        command: command.to_string(),
        code: code.to_string(),
        message: message.to_string(),
        data,
    })
}

fn run_scan_command(args: &[String]) -> Result<String, String> {
    let input = option_value(args, "--input").ok_or_else(|| "missing --input".to_string())?;
    let output = option_value(args, "--output");
    let report = scan_report_json(scan_disk(&input).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    if let Some(output) = output {
        fs::write(&output, &report).map_err(|error| error.to_string())?;
        envelope_json(
            "scan",
            "ok",
            "scan report written",
            Some(serde_json::json!({ "output": output })),
        )
        .map_err(|error| error.to_string())
    } else {
        envelope_json(
            "scan",
            "ok",
            "scan report generated",
            Some(serde_json::json!({ "report": serde_json::from_str::<serde_json::Value>(&report).map_err(|error| error.to_string())? })),
        )
        .map_err(|error| error.to_string())
    }
}

fn run_validate_plan_command(args: &[String]) -> Result<String, String> {
    let plan_path = option_value(args, "--plan").ok_or_else(|| "missing --plan".to_string())?;
    let json = fs::read_to_string(&plan_path).map_err(|error| error.to_string())?;
    let plan = import_recovery_plan_json(&json).map_err(|error| error.to_string())?;
    let errors = plan
        .validation
        .iter()
        .filter(|issue| issue.severity == RecoveryPlanValidationSeverity::Error)
        .count();
    let code = if errors == 0 { "ok" } else { "invalid_plan" };
    envelope_json(
        "validate-plan",
        code,
        if errors == 0 { "plan is valid" } else { "plan contains validation errors" },
        Some(plan),
    )
    .map_err(|error| error.to_string())
}

fn run_format_command(args: &[String]) -> Result<String, String> {
    let target = option_value(args, "--target").ok_or_else(|| "missing --target".to_string())?;
    let filesystem = option_value(args, "--filesystem").ok_or_else(|| "missing --filesystem".to_string())?;
    let name = option_value(args, "--name").ok_or_else(|| "missing --name".to_string())?;
    let confirm = option_value(args, "--confirm").ok_or_else(|| "missing --confirm".to_string())?;
    let unsafe_flag = has_flag(args, "--unsafe");
    let target_disk = resolve_disk(&target).ok_or_else(|| format!("target not found: {target}"))?;
    let filesystem = parse_filesystem(&filesystem)?;
    let request = FormatDiskRequest {
        path: target_disk.path.clone(),
        target: target_disk.clone(),
        filesystem,
        volume_name: name,
        confirmation: confirm,
    };
    if !unsafe_flag {
        let plan = preview_format_plan(&request).map_err(|error| error.to_string())?;
        return envelope_json("format", "ok", "preview-only format plan", Some(plan))
            .map_err(|error| error.to_string());
    }
    let result = format_disk(request).map_err(|error| error.to_string())?;
    envelope_json("format", "ok", "format completed", Some(result)).map_err(|error| error.to_string())
}

fn resolve_disk(identifier: &str) -> Option<testdisk_core::DiskInfo> {
    host_platform_adapter()
        .list_disks()
        .into_iter()
        .find(|disk| disk.path == identifier || disk.platform_id == identifier)
}

fn parse_filesystem(value: &str) -> Result<FormatFilesystem, String> {
    match value.to_ascii_lowercase().as_str() {
        "apfs" => Ok(FormatFilesystem::Apfs),
        "exfat" => Ok(FormatFilesystem::Exfat),
        "fat32" | "ms-dos" | "msdos" => Ok(FormatFilesystem::Fat32),
        "hfs+" | "hfsplus" => Ok(FormatFilesystem::HfsPlus),
        "ntfs" => Ok(FormatFilesystem::Ntfs),
        "ext4" => Ok(FormatFilesystem::Ext4),
        _ => Err(format!("unsupported filesystem: {value}")),
    }
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_lists_filesystems_as_json() {
        let output = dispatch(["mini-testdisk", "list-filesystems"]).unwrap();
        assert!(output.contains("\"command\": \"list-filesystems\""));
        assert!(output.contains("\"ok\": true"));
    }

    #[test]
    fn list_filesystems_response_serializes() {
        let json = list_filesystems_json().unwrap();
        assert!(json.contains("\"command\": \"list-filesystems\""));
        assert!(json.contains("\"ok\": true"));
    }
}
