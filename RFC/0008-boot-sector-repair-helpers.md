# RFC 0008: Boot Sector Repair Helpers

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add read-first helpers for common boot sector and filesystem repair workflows without reimplementing full filesystem repair engines.

## Goals

- Diagnose boot sector problems for supported filesystems.
- Compare primary and backup boot sectors where the filesystem defines them.
- Offer restore plans only when layout and safety are clear.
- Integrate native repair commands as previewable operations.

## Scope

Initial candidates:

- FAT boot sector backup compare/restore.
- NTFS boot sector backup compare/restore.
- ext superblock discovery hints.
- APFS/HFS+ native repair guidance.

## Requirements

- Diagnosis must be available without write access for images.
- Repair plans must be explicit and exported before execution.
- Native commands must be displayed as structured arguments.
- Post-checks must decide whether a repair succeeded.

## Acceptance Criteria

- Read-only diagnosis produces findings and confidence.
- Unsafe or ambiguous layouts do not produce write plans.
- Tests use filesystem fixture images.
- The app never claims repair success without post-repair validation.

## Open Questions

- Which NTFS boot-sector fields are sufficient for safe comparison?
- Should native repair commands run inside the app or be copied for terminal execution first?
