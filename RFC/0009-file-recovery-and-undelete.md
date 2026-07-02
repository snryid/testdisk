# RFC 0009: File Recovery and Undelete

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add file-level recovery workflows for filesystems where metadata can be parsed safely, keeping source disks strictly read-only.

## Goals

- Browse recoverable directory structures where supported.
- List deleted files when filesystem metadata permits.
- Recover selected files to a separate destination.
- Report partial failures per file.

## Non-Goals

- Signature-based file carving; see RFC 0010.
- Writing recovered files back to the source disk.
- Full support for every filesystem in the first version.

## Requirements

- Source target is read-only.
- Destination must not be on the source disk by default.
- Large recovery jobs need progress and cancellation.
- Recovered files should preserve names and directory structure when known.

## Initial Filesystem Candidates

- FAT family: simpler directory metadata.
- ext family: feasible with careful metadata parsing.
- NTFS: later phase due to complexity.
- APFS/HFS+: later phase or native-guided only.

## Acceptance Criteria

- Users can recover files from fixture images to a different directory.
- The app prevents destination paths on the same source target.
- Partial errors are reported per file.
- Recovery never mutates the source.

## Open Questions

- Should filesystem parsers be implemented in-house or use existing Rust crates?
- How should the app handle filename collisions in the destination?
