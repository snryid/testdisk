# RFC 0004: Analysis Parity and Deep Scan Improvements

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Improve read-only analysis so Mini TestDisk can diagnose partition table damage and discover lost partitions with confidence scoring, without performing writes.

## Goals

- Support more partition layouts and inconsistency checks.
- Improve deep scan beyond fixed 1 MB alignment.
- Add confidence scoring and overlap detection.
- Present current vs discovered partition candidates clearly.

## Scope

Implement read-only analysis for:

- MBR primary and extended/logical partitions;
- GPT primary and backup headers;
- hybrid MBR warnings;
- overlapping or out-of-bounds partitions;
- filesystem-backed lost partition candidates.

## Deep Scan Strategy

The scanner should combine:

- known partition table ranges;
- filesystem boot sector signatures;
- backup superblock hints where practical;
- alignment heuristics;
- disk-bound and overlap validation.

Each candidate should include source, confidence, start/end LBA, filesystem hint, and conflict status.

## UI Requirements

- Show current table and discovered candidates side by side.
- Make confidence and conflicts visible.
- Allow exporting scan results as JSON.
- Keep this phase read-only.

## Acceptance Criteria

- Tests cover MBR logical partitions and GPT backup-header detection.
- Lost partition candidates include confidence and conflict information.
- The app can analyze disk images without elevated privileges.
- No write commands are introduced by this RFC.

## Open Questions

- Which filesystem probes should be implemented first after the current signature checks?
- Should deep scan be cancellable before Phase 8 job infrastructure exists?
