use crate::domain::{PartitionTableType, PartitionResult, ScanResult};
use serde::{Deserialize, Serialize};

pub const RECOVERY_PLAN_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPlanValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryPlanValidationIssue {
    pub severity: RecoveryPlanValidationSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryPlanPartition {
    pub index: u32,
    pub name: String,
    pub start_lba: u64,
    pub end_lba: u64,
    pub size_bytes: u64,
    pub type_name: String,
    pub source: String,
    pub status: String,
    pub confidence: u32,
    pub conflict_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryPlanTableMapEntry {
    pub label: String,
    pub start_lba: u64,
    pub end_lba: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryPlan {
    pub schema_version: u32,
    pub created_at_unix_seconds: u64,
    pub app_version: String,
    pub source_disk_path: String,
    pub source_disk_size: u64,
    pub source_disk_signature: String,
    pub original_partition_table_type: PartitionTableType,
    pub proposed_partition_table_type: PartitionTableType,
    pub selected_partitions: Vec<RecoveryPlanPartition>,
    pub before_map: Vec<RecoveryPlanTableMapEntry>,
    pub after_map: Vec<RecoveryPlanTableMapEntry>,
    pub warnings: Vec<String>,
    pub conflicts: Vec<String>,
    pub validation: Vec<RecoveryPlanValidationIssue>,
}

pub fn build_recovery_plan(
    scan: &ScanResult,
    selected_partitions: &[PartitionResult],
    proposed_partition_table_type: PartitionTableType,
    app_version: impl Into<String>,
    created_at_unix_seconds: u64,
) -> Result<RecoveryPlan, Vec<RecoveryPlanValidationIssue>> {
    let selected = normalize_selected_partitions(scan, selected_partitions);
    let mut validation = Vec::new();

    validation.extend(validate_bounds(scan.disk_size, &selected));
    validation.extend(validate_overlaps(&selected));
    validation.extend(validate_table_rules(
        &scan.partition_table_type,
        &proposed_partition_table_type,
        &selected,
        scan.disk_size,
    ));

    let conflicts = selected
        .iter()
        .filter(|partition| partition.conflict_status != "clear")
        .map(|partition| {
            format!(
                "{}:{}",
                partition.name,
                partition.conflict_status.replace('_', "-")
            )
        })
        .collect::<Vec<_>>();

    let warnings = scan.warnings.clone();

    if validation
        .iter()
        .any(|issue| issue.severity == RecoveryPlanValidationSeverity::Error)
    {
        return Err(validation);
    }

    Ok(RecoveryPlan {
        schema_version: RECOVERY_PLAN_SCHEMA_VERSION,
        created_at_unix_seconds,
        app_version: app_version.into(),
        source_disk_path: scan.disk_path.clone(),
        source_disk_size: scan.disk_size,
        source_disk_signature: format!("{}:{}:{}", scan.disk_path, scan.disk_size, scan.partitions.len()),
        original_partition_table_type: scan.partition_table_type.clone(),
        proposed_partition_table_type,
        selected_partitions: selected,
        before_map: build_before_map(scan),
        after_map: build_after_map(selected_partitions),
        warnings,
        conflicts,
        validation,
    })
}

pub fn export_recovery_plan_json(plan: &RecoveryPlan) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(plan)
}

pub fn import_recovery_plan_json(json: &str) -> Result<RecoveryPlan, serde_json::Error> {
    serde_json::from_str(json)
}

fn normalize_selected_partitions(
    scan: &ScanResult,
    selected_partitions: &[PartitionResult],
) -> Vec<RecoveryPlanPartition> {
    let mut selected = selected_partitions
        .iter()
        .map(|partition| RecoveryPlanPartition {
            index: partition.index,
            name: partition.name.clone(),
            start_lba: partition.start_lba,
            end_lba: partition.end_lba,
            size_bytes: partition.size_bytes,
            type_name: partition.type_name.clone(),
            source: partition.source.clone(),
            status: partition.status.clone(),
            confidence: (partition.confidence * 1000.0).round() as u32,
            conflict_status: partition.conflict_status.clone(),
        })
        .collect::<Vec<_>>();

    selected.sort_by(|left, right| {
        left.start_lba
            .cmp(&right.start_lba)
            .then(left.end_lba.cmp(&right.end_lba))
            .then(left.index.cmp(&right.index))
    });

    if selected.is_empty() {
        selected.extend(scan.partitions.iter().map(|partition| RecoveryPlanPartition {
            index: partition.index,
            name: partition.name.clone(),
            start_lba: partition.start_lba,
            end_lba: partition.end_lba,
            size_bytes: partition.size_bytes,
            type_name: partition.type_name.clone(),
            source: partition.source.clone(),
            status: partition.status.clone(),
            confidence: (partition.confidence * 1000.0).round() as u32,
            conflict_status: partition.conflict_status.clone(),
        }));
    }

    selected
}

fn build_before_map(scan: &ScanResult) -> Vec<RecoveryPlanTableMapEntry> {
    scan.partitions
        .iter()
        .chain(scan.lost_partitions.iter())
        .map(|partition| RecoveryPlanTableMapEntry {
            label: format!("{}:{}", partition.index, partition.name),
            start_lba: partition.start_lba,
            end_lba: partition.end_lba,
        })
        .collect()
}

fn build_after_map(selected: &[PartitionResult]) -> Vec<RecoveryPlanTableMapEntry> {
    let mut after = selected
        .iter()
        .map(|partition| RecoveryPlanTableMapEntry {
            label: format!("{}:{}", partition.index, partition.name),
            start_lba: partition.start_lba,
            end_lba: partition.end_lba,
        })
        .collect::<Vec<_>>();
    after.sort_by(|left, right| {
        left.start_lba
            .cmp(&right.start_lba)
            .then(left.end_lba.cmp(&right.end_lba))
            .then(left.label.cmp(&right.label))
    });
    after
}

fn validate_bounds(
    disk_size: u64,
    selected: &[RecoveryPlanPartition],
) -> Vec<RecoveryPlanValidationIssue> {
    let max_lba = disk_size.saturating_div(512).saturating_sub(1);
    selected
        .iter()
        .flat_map(|partition| {
            let mut issues = Vec::new();
            if partition.end_lba < partition.start_lba {
                issues.push(issue_error(
                    "invalid-range",
                    format!("{} has end_lba before start_lba", partition.name),
                ));
                return issues;
            }
            if partition.end_lba > max_lba {
                issues.push(issue_error(
                    "out-of-bounds",
                    format!("{} extends beyond disk size", partition.name),
                ));
            }
            issues
        })
        .collect()
}

fn validate_overlaps(selected: &[RecoveryPlanPartition]) -> Vec<RecoveryPlanValidationIssue> {
    let mut issues = Vec::new();
    for window in selected.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if right.start_lba <= left.end_lba {
            issues.push(issue_error(
                "overlap",
                format!("{} overlaps {}", left.name, right.name),
            ));
        }
    }
    issues
}

fn validate_table_rules(
    original: &PartitionTableType,
    proposed: &PartitionTableType,
    selected: &[RecoveryPlanPartition],
    disk_size: u64,
) -> Vec<RecoveryPlanValidationIssue> {
    let mut issues = Vec::new();
    let max_lba = disk_size.saturating_div(512).saturating_sub(1);

    match proposed {
        PartitionTableType::Gpt => {
            if selected.iter().any(|partition| partition.start_lba < 34) {
                issues.push(issue_error(
                    "gpt-reserved-space",
                    "GPT plans must leave the primary header and partition array space intact".to_string(),
                ));
            }
            if selected.iter().any(|partition| partition.end_lba > max_lba.saturating_sub(33)) {
                issues.push(issue_error(
                    "gpt-backup-space",
                    "GPT plans must leave backup header and partition array space intact".to_string(),
                ));
            }
        }
        PartitionTableType::Mbr => {
            let primary_count = selected
                .iter()
                .filter(|partition| partition.source != "mbr-logical")
                .count();
            if primary_count > 4 {
                issues.push(issue_error(
                    "mbr-primary-count",
                    "MBR plans can only contain four primary entries".to_string(),
                ));
            }
            if selected
                .iter()
                .any(|partition| partition.source == "mbr-logical")
                && !selected.iter().any(|partition| {
                    partition.source == "mbr" && partition.type_name.contains("Extended")
                })
            {
                issues.push(issue_error(
                    "mbr-logical-container",
                    "Logical partitions require an extended container entry".to_string(),
                ));
            }
        }
        PartitionTableType::Unknown => {
            issues.push(issue_warning(
                "unknown-table",
                "The source table type is unknown; proposed layout validation is limited".to_string(),
            ));
        }
    }

    if matches!(original, PartitionTableType::Unknown)
        && !matches!(proposed, PartitionTableType::Unknown)
    {
        issues.push(issue_warning(
            "type-inference",
            "The scan result did not prove the original table type; proposal is inferred from selection".to_string(),
        ));
    }

    issues
}

fn issue_error(code: impl Into<String>, message: String) -> RecoveryPlanValidationIssue {
    RecoveryPlanValidationIssue {
        severity: RecoveryPlanValidationSeverity::Error,
        code: code.into(),
        message,
    }
}

fn issue_warning(code: impl Into<String>, message: String) -> RecoveryPlanValidationIssue {
    RecoveryPlanValidationIssue {
        severity: RecoveryPlanValidationSeverity::Warning,
        code: code.into(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scan_result() -> ScanResult {
        ScanResult {
            disk_path: "/tmp/test.img".to_string(),
            disk_size: 1024 * 1024 * 1024,
            partition_table_type: PartitionTableType::Gpt,
            mbr: None,
            gpt: None,
            partitions: vec![PartitionResult {
                index: 1,
                name: "System".to_string(),
                start_lba: 2048,
                end_lba: 4095,
                size_bytes: 2048 * 512,
                type_name: "EFI System".to_string(),
                filesystem: None,
                status: "known".to_string(),
                source: "gpt".to_string(),
                confidence: 0.95,
                conflict_status: "clear".to_string(),
            }],
            lost_partitions: vec![],
            warnings: vec!["scan warning".to_string()],
        }
    }

    #[test]
    fn exports_recovery_plan_deterministically_for_fixed_timestamp() {
        let scan = scan_result();
        let plan = build_recovery_plan(
            &scan,
            &scan.partitions,
            PartitionTableType::Gpt,
            "0.1.0",
            12345,
        )
        .unwrap();

        assert_eq!(plan.schema_version, RECOVERY_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.created_at_unix_seconds, 12345);
        assert_eq!(plan.selected_partitions.len(), 1);
        assert_eq!(plan.before_map.len(), 1);
        assert!(plan.validation.iter().all(|issue| issue.severity != RecoveryPlanValidationSeverity::Error));

        let json = export_recovery_plan_json(&plan).unwrap();
        let imported = import_recovery_plan_json(&json).unwrap();
        assert_eq!(imported, plan);
    }

    #[test]
    fn rejects_overlapping_mbr_plan() {
        let mut scan = scan_result();
        scan.partition_table_type = PartitionTableType::Mbr;
        scan.partitions.push(PartitionResult {
            index: 2,
            name: "Data".to_string(),
            start_lba: 3000,
            end_lba: 5000,
            size_bytes: 2001 * 512,
            type_name: "Linux".to_string(),
            filesystem: None,
            status: "known".to_string(),
            source: "mbr".to_string(),
            confidence: 0.9,
            conflict_status: "clear".to_string(),
        });
        let err = build_recovery_plan(
            &scan,
            &scan.partitions,
            PartitionTableType::Mbr,
            "0.1.0",
            12345,
        )
        .unwrap_err();
        assert!(err.iter().any(|issue| issue.code == "overlap"));
    }
}
