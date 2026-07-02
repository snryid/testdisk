# RFC 0001: Mini TestDisk Cross-Platform Roadmap

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

This RFC defines a phased roadmap for evolving Mini TestDisk from the current Tauri + Rust feasibility prototype into a cross-platform disk analysis, recovery, and maintenance tool inspired by TestDisk. The roadmap keeps every feature portable across macOS, Linux, and Windows, while isolating platform-specific disk access behind explicit adapters.

The project should not attempt to clone every TestDisk and PhotoRec capability at once. The first goal is a safe, testable architecture that supports read-only analysis everywhere, then constrained write operations, then deeper recovery workflows.

## Goals

- Provide a cross-platform GUI for disk, partition, and filesystem inspection.
- Support physical disks and disk images on macOS, Linux, and Windows.
- Add recovery and repair capabilities in phases with explicit safety gates.
- Keep destructive operations isolated, auditable, and guarded by confirmation.
- Build reusable core logic in Rust, with thin platform adapters for device enumeration, permissions, mounting, formatting, and write operations.

## Non-Goals

- No blind cloning of TestDisk's ncurses workflow.
- No unrestricted write access to system disks.
- No kernel extensions or custom drivers in the initial roadmap.
- No proprietary filesystem repair implementations when the host platform already provides safer native tooling.
- No PhotoRec-level file carving in the same milestone as partition repair.

## Current State

Current implemented capabilities:

- Tauri desktop shell with Vue frontend.
- Disk/image selection UI.
- macOS/Linux-oriented disk enumeration.
- Disk image opening.
- MBR and GPT detection.
- GPT header CRC validation.
- Filesystem signature detection for common filesystems.
- Simplified 1 MB-aligned lost partition scan.
- macOS external/USB whole-disk formatting with APFS, ExFAT, FAT32, and HFS+.

Current gaps:

- Windows disk enumeration and raw disk access are not implemented.
- Linux formatting and privilege handling are not implemented.
- Disk metadata is minimal; internal/external/removable/protocol information is not modeled consistently.
- No persistent scan reports.
- No partition table reconstruction proposal workflow.
- No write-back repair path.
- No boot sector backup/restore.
- No undelete/file recovery.
- No SMART or imaging workflow.
- No scripted or batch mode.

## Design Principles

### Cross-Platform First

Every user-visible feature must define behavior for macOS, Linux, and Windows before implementation starts. If a capability depends on platform tools, the RFC for that feature must list the command/API per OS and the unsupported-case behavior.

### Read Before Write

Each write feature must have a read-only preview mode first. The user must be able to see the exact target, planned action, risk level, and expected result before any disk mutation.

### Explicit Device Model

The core should model disks, partitions, volumes, and images separately:

- Disk: whole physical or virtual device.
- Partition: partition-table entry with byte/LBA range.
- Volume: mountable filesystem instance.
- Image: regular file treated as a disk.

### Native Adapters, Shared Core

Partition parsing, signature detection, scan planning, and result modeling should live in `testdisk-core`. OS-specific access should live behind adapter traits used by the Tauri command layer or a dedicated platform crate.

### Safety by Default

Dangerous operations must require:

- target classification,
- non-system-disk guard,
- dry-run plan,
- typed confirmation,
- operation log,
- post-action verification.

## Proposed Architecture

```text
ui/
  Vue workflow panels
  scan results
  recovery plans
  destructive-operation confirmation

src-tauri/
  IPC commands
  platform adapter selection
  privilege and native command orchestration

crates/testdisk-core/
  partition parsers
  filesystem detectors
  scan engines
  recovery plan models
  validation and safety checks

crates/testdisk-platform/      [new]
  macOS diskutil adapter
  Linux lsblk/blkid/mkfs adapter
  Windows WMI/PowerShell/Win32 adapter
```

## Platform Adapter Targets

| Capability | macOS | Linux | Windows |
|------------|-------|-------|---------|
| Disk enumeration | `diskutil list/info -plist` | `lsblk --json`, `/sys/block` | WMI / PowerShell `Get-Disk`, Win32 APIs |
| Raw read | `/dev/rdiskN` | `/dev/sdX`, `/dev/nvmeXnY` | `\\.\PhysicalDriveN` |
| Filesystem metadata | `diskutil info -plist` | `blkid`, `lsblk -f` | WMI / PowerShell `Get-Volume` |
| Mount/unmount | `diskutil mount/unmountDisk` | `udisksctl`, `mount`, `umount` | PowerShell `Mount-Volume`, `Dismount-Volume` |
| Format | `diskutil eraseDisk/eraseVolume` | `mkfs.*`, `parted` | PowerShell `Format-Volume`, `Clear-Disk` |
| SMART | `smartctl` optional | `smartctl` optional | `smartctl` optional / WMI limited |
| Imaging | core Rust reader/writer | core Rust reader/writer | core Rust reader/writer |

## Phased Roadmap

### Phase 0: Foundation Hardening

Purpose: make the current prototype reliable as a cross-platform base.

Features:

- Add a normalized disk model with fields for source, platform id, display name, protocol, internal/external, removable, size, read/write capability, and risk level.
- Split platform access out of `disk.rs` into a platform adapter boundary.
- Add Windows disk/image read support.
- Add Linux disk metadata and permission detection.
- Add consistent error types for permission, device busy, unsupported platform, invalid target, and command failure.
- Add scan report export as JSON.

Acceptance criteria:

- The same UI can enumerate disks on macOS, Linux, and Windows.
- Disk images work on all three platforms.
- Unit tests cover target normalization and safety classification.
- No write operations are added in this phase.

### Phase 1: Analysis Parity

Purpose: improve TestDisk-like read-only diagnosis.

Features:

- Expand partition table support: MBR, GPT, hybrid MBR, extended/logical partitions.
- Detect partition table inconsistencies and produce warnings.
- Detect backup GPT and compare primary vs backup headers.
- Improve deep scan beyond fixed 1 MB boundaries.
- Add confidence scoring for discovered partitions.
- Add detailed filesystem probes for FAT12/16/32, exFAT, NTFS, ext2/3/4, APFS, HFS/HFS+, UFS where feasible.
- Add side-by-side "current table vs discovered candidates" UI.

Acceptance criteria:

- Scan results include confidence, source, overlap status, and repair recommendation.
- Deep scan can find partitions not aligned to exactly 1 MB when filesystem signatures are present.
- Read-only analysis remains available without elevated privileges for image files.

### Phase 2: Safe Formatting and Volume Operations

Purpose: complete the formatting work started in the prototype across platforms.

Features:

- Cross-platform external/removable whole-disk formatting.
- Cross-platform partition/volume formatting where supported.
- Filesystem list filtered by platform and target type.
- Unmount-before-format workflow.
- Post-format verification and automatic rescan.
- Operation log stored as JSON.

Filesystem target matrix:

| Filesystem | macOS | Linux | Windows |
|------------|-------|-------|---------|
| APFS | yes | no native format | no |
| HFS+ | yes | optional tools | no |
| ExFAT | yes | yes with tools | yes |
| FAT32 | yes | yes | yes |
| NTFS | no native safe format | optional tools | yes |
| ext4 | no | yes | no |

Acceptance criteria:

- UI never offers a filesystem unsupported by the current platform adapter.
- Internal/system disks are blocked by backend validation.
- Formatting requires typed confirmation and returns a structured operation result.

### Phase 3: Recovery Plan Builder

Purpose: let users build a partition recovery proposal without writing it yet.

Features:

- Select discovered partitions to include in a recovery plan.
- Validate overlaps, ordering, disk bounds, and partition table constraints.
- Generate a proposed MBR or GPT table in memory.
- Show before/after partition map.
- Export recovery plan JSON.
- Import recovery plan JSON for review.

Acceptance criteria:

- No disk writes occur in this phase.
- Invalid plans are rejected with actionable messages.
- Plans are deterministic and covered by snapshot tests.

### Phase 4: Partition Table Write-Back

Purpose: implement TestDisk-style partition table repair with strong guardrails.

Features:

- Write MBR or GPT recovery plan to selected target.
- Create mandatory backup of original sectors before writing.
- Support restore-from-backup.
- Require offline/unmounted target where applicable.
- Verify written sectors by rereading and comparing.
- Keep full operation transcript.

Acceptance criteria:

- Write-back is unavailable until a valid Phase 3 plan exists.
- The app refuses to write to mounted/system disks unless a platform adapter proves it is safe.
- Recovery backup can restore original sectors in tests using disk images.

### Phase 5: Boot Sector and Filesystem Repair Helpers

Purpose: add common TestDisk repair helpers without reimplementing full filesystem repair engines.

Features:

- FAT boot sector backup compare/restore where layout is well understood.
- NTFS boot sector backup compare/restore where safe.
- ext superblock discovery hints.
- APFS/HFS+ repair guidance through native tooling.
- "Run native repair command" adapter hooks with preview.

Acceptance criteria:

- Every repair helper has a read-only diagnosis mode.
- Any native repair command is displayed before execution.
- The app does not claim success unless post-checks pass.

### Phase 6: File Recovery and Undelete

Purpose: add file-level recovery workflows after disk/partition workflows are stable.

Features:

- Directory tree browsing for supported filesystems where metadata parsing is implemented.
- Deleted file listing for filesystems where safe metadata interpretation is feasible.
- Recover selected files to a different destination disk.
- Never recover files onto the source disk by default.

Acceptance criteria:

- Destination path must be different from source target.
- Recovery is read-only against the source disk.
- Partial recovery errors are reported per file.

### Phase 7: PhotoRec-Style File Carving

Purpose: add signature-based carving as a separate workflow from partition recovery.

Features:

- Scan raw sectors for file signatures.
- Select file type families.
- Recover carved files to destination directory.
- Resume interrupted carving jobs.
- Progress, throughput, and estimated remaining time.

Acceptance criteria:

- Carving does not depend on valid partition tables.
- Output directory must be outside the source disk.
- Job state can resume after app restart.

### Phase 8: Imaging, SMART, and Triage

Purpose: support safer recovery workflows before mutation.

Features:

- Create raw disk image from physical disk.
- Verify image hash.
- Resume interrupted imaging.
- Optional integration with `ddrescue` where installed.
- SMART health summary where `smartctl` or native APIs are available.
- Warn when the source disk looks physically unhealthy.

Acceptance criteria:

- The UI recommends imaging before repair if SMART or read errors indicate risk.
- Image files can be scanned immediately after creation.
- Hash and read-error summaries are stored in report JSON.

### Phase 9: Automation and Advanced Workflows

Purpose: make the tool usable for repeatable support workflows.

Features:

- CLI wrapper for scan, report export, and plan validation.
- Batch image scanning.
- Headless JSON output.
- Scripted recovery plan application with explicit unsafe flag.
- Localized UI strings.

Acceptance criteria:

- GUI and CLI share the same core logic.
- CLI destructive operations require explicit flags and confirmation tokens.
- Batch mode never performs write operations by default.

## UI Roadmap

The UI should evolve from the current two-table layout into task-focused tabs:

- Devices: list disks/images, metadata, health, permissions.
- Analyze: partition table, warnings, discovered candidates.
- Recover: recovery plan builder and write-back.
- Format: external/removable formatting and volume operations.
- Image: create/verify/resume disk images.
- Files: undelete and carving workflows.
- Reports: export/import scan and operation reports.

## Testing Strategy

Required test layers:

- Unit tests for parsers, filesystem detectors, target normalization, and safety gates.
- Fixture tests using generated MBR/GPT/APFS/FAT/exFAT/ext images.
- Platform adapter tests with command-output fixtures instead of real device mutation.
- Destructive operation tests only against temporary disk images or mocked command runners.
- Manual hardware test matrix for USB disks on macOS, Linux, and Windows.

Minimum CI matrix:

- macOS latest: unit tests, Vue build, Tauri compile.
- Ubuntu latest: unit tests, Vue build, Tauri compile.
- Windows latest: unit tests, Vue build, Tauri compile.

## Security and Safety Requirements

- Never infer that a disk is safe to write because the frontend says so.
- Backend must classify the target before every destructive command.
- Internal/system disks are blocked unless a future RFC defines an explicit expert override.
- Every write operation must produce an operation log.
- Every write operation must have a dry-run representation.
- Every recovery output path must be different from the source disk.
- No destructive command should be constructed with shell interpolation; use structured process arguments.

## RFC Breakdown

This document is the roadmap RFC. Each implementation phase should get its own follow-up RFC before coding:

- RFC 0002: Platform Adapter Boundary and Disk Model
- RFC 0003: Cross-Platform Disk Enumeration
- RFC 0004: Analysis Parity and Deep Scan Improvements
- RFC 0005: Cross-Platform Formatting
- RFC 0006: Recovery Plan Builder
- RFC 0007: Partition Table Write-Back and Backup
- RFC 0008: Boot Sector Repair Helpers
- RFC 0009: File Recovery and Undelete
- RFC 0010: PhotoRec-Style File Carving
- RFC 0011: Imaging, SMART, and Triage
- RFC 0012: CLI and Batch Automation

## Open Questions

- Should Linux formatting depend on system `mkfs.*` tools, bundled helpers, or both?
- Should Windows raw disk access use PowerShell only at first, or direct Win32 APIs?
- What is the minimum supported macOS version for APFS workflows?
- Should expert override for internal disks ever exist?
- Should PhotoRec-style carving live in this repo or a separate crate?

## References

- TestDisk official documentation: https://www.cgsecurity.org/testdisk_doc/
- TestDisk wiki: https://www.cgsecurity.org/wiki/TestDisk
- TestDisk source repository: https://github.com/cgsecurity/testdisk
- Tauri documentation: https://tauri.app/
