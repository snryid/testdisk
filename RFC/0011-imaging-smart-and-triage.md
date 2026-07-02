# RFC 0011: Imaging, SMART, and Triage

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add disk imaging, health checks, and triage guidance so users can work from images before attempting risky recovery operations.

## Goals

- Create raw images from physical disks.
- Verify image hashes.
- Resume interrupted imaging jobs.
- Surface SMART or platform health signals where available.
- Recommend imaging before repair when risk is high.

## Imaging Requirements

- Read source in chunks.
- Track bytes copied, read errors, and throughput.
- Store image metadata and hash.
- Support cancellation and resume.
- Never write output to the source disk.

## Health Sources

| Platform | Health source |
|----------|---------------|
| macOS | `diskutil`, optional `smartctl` |
| Linux | `smartctl`, `/sys/block` |
| Windows | WMI / PowerShell, optional `smartctl` |

## Triage Rules

The UI should recommend image-first workflows when:

- SMART reports failure or pre-failure indicators;
- repeated read errors occur;
- the disk disconnects or changes identity;
- scan speed drops unexpectedly.

## Acceptance Criteria

- Images can be created from fixture devices or test files.
- Hash and read-error summaries are stored in JSON.
- Interrupted jobs can resume after source and output verification.
- Risk warnings appear before repair/write actions.

## Open Questions

- Should `ddrescue` be integrated as an optional external tool?
- Which hash algorithm should be default for large images?
