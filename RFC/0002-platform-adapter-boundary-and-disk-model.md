# RFC 0002: Platform Adapter Boundary and Disk Model

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Define a stable disk model and platform adapter boundary so Mini TestDisk can support macOS, Linux, and Windows without spreading OS-specific logic through the core scanner and UI.

## Goals

- Separate shared disk analysis logic from platform-specific device access.
- Model disks, partitions, volumes, and images consistently.
- Provide a safety classification that all destructive operations must use.
- Keep platform command output parsing testable with fixtures.

## Non-Goals

- Implement full disk enumeration for every platform in this RFC.
- Add new write operations.
- Replace the current scanner algorithms.

## Proposed Model

Core types should represent:

- `DiskTarget`: whole physical disk, virtual disk, or image file.
- `PartitionTarget`: partition-table entry with start/end LBA and byte range.
- `VolumeTarget`: mountable filesystem instance.
- `TargetSafety`: system, internal, external, removable, image, unknown.
- `AccessCapability`: readable, writable, requires elevation, busy, unsupported.

## Adapter Boundary

Introduce a platform adapter trait with methods for:

- listing disks;
- resolving a user-selected path into a normalized target;
- reading disk metadata;
- checking mount state;
- classifying safety;
- producing platform-specific operation plans.

`testdisk-core` should keep parsers and scan logic. Platform command execution should live outside pure parsing logic.

## Cross-Platform Requirements

| Area | macOS | Linux | Windows |
|------|-------|-------|---------|
| Disk identity | `diskN`, `/dev/rdiskN` | `/dev/sdX`, `/dev/nvmeXnY` | `PhysicalDriveN` |
| Metadata | `diskutil info -plist` | `lsblk --json`, `/sys/block` | WMI / PowerShell |
| Safety | internal/removable/protocol | removable/rotational/model | bus type, boot/system flags |

## Acceptance Criteria

- Disk and image targets use one shared model in frontend and backend.
- Platform-specific identifiers are preserved but not used as the only identity.
- Unit tests cover normalization for macOS, Linux, and Windows sample targets.
- Safety classification is produced by backend code, never inferred by the frontend.

## Open Questions

- Should platform adapters live in `crates/testdisk-platform` immediately or start inside `src-tauri`?
- Should command execution use sync process calls or an async job runner from the start?
