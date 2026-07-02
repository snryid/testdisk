# RFC 0003: Cross-Platform Disk Enumeration

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Implement disk and image enumeration consistently across macOS, Linux, and Windows, including metadata required for analysis and safety decisions.

## Goals

- List physical disks, removable disks, virtual disks, and images.
- Include size, display name, platform id, protocol, mount state, and access capability.
- Detect permission problems without treating them as empty disk lists.
- Provide predictable UI behavior on every supported platform.

## Platform Plan

| Platform | Primary source | Fallback |
|----------|----------------|----------|
| macOS | `diskutil list -plist`, `diskutil info -plist` | `/dev/disk*` probing |
| Linux | `lsblk --json --bytes --output ...` | `/sys/block` and `blkid` |
| Windows | PowerShell `Get-Disk`, `Get-Partition`, `Get-Volume` | WMI / Win32 APIs |

## Data Contract

Each listed item should include:

- normalized id;
- display path;
- raw read path;
- size in bytes;
- device kind;
- bus/protocol;
- removable/external flag;
- mount state;
- readable/writable status;
- permission message if unavailable.

## UI Requirements

- Empty list and permission-denied states must be visually different.
- Unreadable disks remain selectable for diagnostics, but destructive actions stay blocked.
- Images should be clearly labeled separately from physical disks.

## Acceptance Criteria

- Disk enumeration works on macOS, Linux, and Windows in CI or fixture tests.
- Permission-denied disks show actionable messages.
- USB/external disks can be distinguished from internal disks where the platform provides metadata.
- The scanner can open image files on all platforms.

## Risks

- Windows raw disk access may require administrator rights.
- Linux device naming differs across SATA, NVMe, loop, and removable media.
- macOS synthesized APFS containers can confuse physical disk vs volume modeling.
