# RFC 0010: PhotoRec-Style File Carving

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add signature-based file carving as a separate recovery workflow that does not depend on partition tables or filesystem metadata.

## Goals

- Scan raw bytes for known file signatures.
- Let users select file type families.
- Recover carved files to a safe destination.
- Support progress, cancellation, and resume.

## Non-Goals

- Reconstruct original directory structure.
- Guarantee original filenames.
- Replace metadata-based undelete workflows.

## Carving Pipeline

The job pipeline should include:

- target selection;
- output directory validation;
- file signature family selection;
- chunked raw scan;
- candidate validation;
- output writing;
- resumable job state.

## Safety Requirements

- Output directory must be outside the source disk.
- Source target remains read-only.
- Job state must not contain raw recovered file content.
- Resume must verify source identity before continuing.

## Acceptance Criteria

- Carving works against raw image fixtures.
- Users can pause/cancel long scans.
- Recovered files are grouped by type.
- The app can resume a job after restart when source identity matches.

## Open Questions

- Should signature definitions be built in, user-editable, or both?
- Should carving run in a separate process to isolate crashes and memory use?
