# RFC 0007: Partition Table Write-Back and Backup

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Implement guarded write-back of validated recovery plans, including mandatory sector backups and post-write verification.

## Goals

- Apply a validated RFC 0006 recovery plan to a disk or image.
- Back up original sectors before writing.
- Support restore from backup.
- Verify written sectors by rereading.
- Keep a complete operation transcript.

## Safety Requirements

- Write-back requires a valid recovery plan.
- The target must match the plan identity.
- Mounted/system/internal disks are blocked unless a future expert-mode RFC changes this.
- The user must pass typed confirmation and native confirmation.
- Image targets are preferred for automated tests.

## Write Plan

The backend should produce a dry-run plan containing:

- target disk;
- sectors to read for backup;
- sectors to write;
- partition table type;
- expected hash of planned bytes;
- rollback backup path.

## Acceptance Criteria

- Tests can write and restore MBR/GPT tables on temporary image files.
- Physical disk write-back is disabled unless target safety checks pass.
- Backup restore returns the image to its original sector bytes.
- Operation logs include plan, confirmation, write result, and verification result.

## Open Questions

- Should backup files be stored beside exported recovery plans or in an app data directory?
- Should GPT write-back update both primary and backup tables in one transaction plan?
