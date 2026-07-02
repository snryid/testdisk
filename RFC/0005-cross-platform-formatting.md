# RFC 0005: Cross-Platform Formatting

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Extend formatting from macOS external disks to a cross-platform, safety-gated formatting workflow for external/removable disks and supported volumes.

## Goals

- Support external/removable whole-disk formatting on macOS, Linux, and Windows.
- Offer only filesystems supported by the active platform and target.
- Add unmount, format, verify, and rescan steps.
- Log every destructive operation.

## Filesystem Matrix

| Filesystem | macOS | Linux | Windows |
|------------|-------|-------|---------|
| APFS | yes | no | no |
| HFS+ | yes | optional | no |
| ExFAT | yes | yes | yes |
| FAT32 | yes | yes | yes |
| NTFS | no native safe format | optional | yes |
| ext4 | no | yes | no |

## Backend Requirements

- Classify target before every format command.
- Reject internal/system/unknown disks.
- Reject partition targets for whole-disk format.
- Use structured process arguments, not shell interpolation.
- Store operation plan and result.

## UI Requirements

- Show target, filesystem, volume name, and risk before execution.
- Require typed confirmation matching the target id.
- Require native confirmation dialog.
- Refresh disk metadata after completion.

## Acceptance Criteria

- Formatting tests use mocked command runners or temporary images only.
- Unsupported filesystems never appear in the UI.
- Backend blocks unsafe targets even if the frontend is bypassed.
- Operation result includes command plan, status, stdout/stderr summary, and post-check status.

## Open Questions

- Should Linux depend on installed `mkfs.*` tools or bundled helper binaries?
- Should Windows use PowerShell first or direct Win32 APIs?
