# RFC 0006: Recovery Plan Builder

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add a read-only workflow for building a partition recovery plan from scan results before any write-back feature exists.

## Goals

- Let users select discovered partition candidates.
- Validate a proposed MBR or GPT layout in memory.
- Export and import recovery plans as JSON.
- Provide a before/after partition map.

## Non-Goals

- Write partition tables to disk.
- Repair boot sectors or filesystems.
- Recover files.

## Plan Model

A recovery plan should include:

- source disk identity and size;
- original partition table summary;
- selected candidates;
- proposed table type;
- validation results;
- warnings and conflicts;
- creation timestamp and app version.

## Validation Rules

- Partitions must not overlap.
- Partitions must remain inside disk bounds.
- GPT plans must reserve required header and table space.
- MBR plans must respect primary/logical partition constraints.
- Filesystem hints must not be treated as proof of correctness.

## Acceptance Criteria

- Users can build and export a plan without write permissions.
- Invalid plans are rejected with actionable diagnostics.
- Plan generation is deterministic for the same scan result.
- Snapshot tests cover representative MBR and GPT plans.

## Open Questions

- Should imported plans be tied to disk serial/hash to reduce accidental reuse?
- Should the UI allow manual LBA editing in the first version?
