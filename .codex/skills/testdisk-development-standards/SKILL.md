---
name: testdisk-development-standards
description: Use when designing or implementing Mini TestDisk features, refactoring Rust modules, changing Vue UI, adding themes, adding localization, or updating RFC-driven work in this repository.
---

# Mini TestDisk Development Standards

## Overview

All future Mini TestDisk work must follow two project standards: Rust code uses enterprise-style layered boundaries, and the frontend is human-centered with theme and language support. Apply this before writing implementation plans, RFCs, or code.

## Required Before Coding

- Read the relevant `RFC/` document first; for Chinese context, read `RFC/zh-CN/`.
- Keep platform-specific logic out of shared parsing/scanning code.
- Design UI states for light theme, dark theme, Chinese, and English before editing components.
- Add tests for model boundaries, safety checks, and serialization contracts when changing Rust types.

## Rust Architecture Standard

Use layered modules. Do not merge platform access, domain parsing, command orchestration, and UI DTOs into one file.

Preferred boundaries:

- `domain`: stable business types such as disks, partitions, filesystems, scan reports, safety classifications.
- `scanner`: read-only scan engines and filesystem/partition detectors.
- `platform`: macOS/Linux/Windows adapters, native command parsing, permission/mount metadata.
- `operations`: destructive or stateful workflows such as format, write-back, imaging, recovery jobs.
- `ports`: traits/interfaces between domain logic and platform/operation implementations.
- `reporting`: JSON schemas, export/import, operation logs.
- `tauri/api`: IPC DTOs and command handlers only; no parsing algorithms or platform command construction.

Rules:

- Shared core code must be deterministic and testable with fixtures.
- Platform commands must use structured process arguments, never shell string interpolation.
- Destructive operations must consume backend safety classifications; never trust frontend safety flags.
- Avoid free-form status strings for contracts; prefer `serde` enums using `snake_case`.
- Files over roughly 300 lines should be reviewed for split opportunities before adding more behavior.

## Frontend UI Standard

The UI should feel like a practical recovery workstation, not a developer demo.

Requirements:

- Provide light and dark themes with persistent user preference.
- Provide Chinese and English language switching with persistent user preference.
- Do not hardcode visible strings inside workflows once i18n exists; route through a translation map or i18n module.
- Show empty, loading, permission denied, device busy, unsupported platform, and destructive-risk states distinctly.
- Keep destructive actions visually isolated from read-only analysis.
- Prefer task-oriented areas: Devices, Analyze, Recover, Format, Image, Files, Reports.
- Metadata panels should be scannable: target identity, safety, access, platform, size, warnings.
- Check narrow desktop/mobile widths so controls do not overlap or truncate important disk paths.

## RFC Update Standard

When adding requirements that affect all future work:

- Update `RFC/0001-cross-platform-testdisk-roadmap.md`.
- Update `RFC/zh-CN/0001-cross-platform-testdisk-roadmap.md`.
- If the requirement belongs to a later phase, update that numbered RFC pair too.
- Keep English and Chinese RFCs structurally aligned.

## Verification

For implementation changes, run:

```bash
npm run build
cargo test --workspace --target-dir ./target
```

If local `target/` contains root-owned files from `sudo make dev`, clean or repair the directory before using this command.
