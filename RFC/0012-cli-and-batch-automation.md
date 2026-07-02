# RFC 0012: CLI and Batch Automation

Status: Accepted
Created: 2026-07-02
Owner: Beck

## Summary

Add a CLI and batch automation layer that reuses the same Rust core as the GUI while keeping destructive operations disabled by default.

## Goals

- Provide headless scan and report export.
- Validate recovery plans from the command line.
- Batch scan disk images.
- Support scripted operations with explicit unsafe flags.
- Share logic between GUI and CLI.

## Commands

Initial commands:

- `mini-testdisk scan --input <path> --output report.json`
- `mini-testdisk validate-plan --plan plan.json`
- `mini-testdisk list-filesystems`
- `mini-testdisk format --target <id> --filesystem <fs> --name <name> --confirm <id> --unsafe`

## Safety Requirements

- CLI write operations require explicit unsafe flags.
- Dry-run is default for destructive commands.
- Confirmation tokens are required even in scripts.
- JSON output must include machine-readable error codes.

## Batch Requirements

- Batch mode scans images only in the first version.
- Each input produces an independent result.
- One failed input must not stop unrelated inputs unless `--fail-fast` is set.
- Summary output lists successes, warnings, and failures.

## Acceptance Criteria

- CLI and GUI call the same core APIs.
- Scan JSON schema is documented.
- Batch scans are deterministic and resumable at file granularity.
- Destructive CLI paths are covered by dry-run tests.

## Open Questions

- Should the CLI ship as a separate binary or be a Tauri sidecar?
- Should JSON schemas be versioned independently from the app version?
