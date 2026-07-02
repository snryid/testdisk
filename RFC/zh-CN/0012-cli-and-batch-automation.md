# RFC 0012：CLI 和批处理自动化

状态：通过
创建日期：2026-07-02
负责人：Beck

## 摘要

增加 CLI 和批处理自动化层，与 GUI 复用同一 Rust 核心，同时默认禁用破坏性操作。

## 目标

- 提供无头扫描和报告导出。
- 从命令行校验恢复计划。
- 批量扫描磁盘镜像。
- 使用显式 unsafe 标志支持脚本化操作。
- GUI 和 CLI 共享逻辑。

## 命令

初始命令：

- `mini-testdisk scan --input <path> --output report.json`
- `mini-testdisk validate-plan --plan plan.json`
- `mini-testdisk list-filesystems`
- `mini-testdisk format --target <id> --filesystem <fs> --name <name> --confirm <id> --unsafe`

## 安全要求

- CLI 写入操作必须要求显式 unsafe 标志。
- 破坏性命令默认 dry-run。
- 即使在脚本中也必须提供确认令牌。
- JSON 输出必须包含机器可读错误码。

## 批处理要求

- 第一版批处理只扫描镜像文件。
- 每个输入生成独立结果。
- 除非设置 `--fail-fast`，单个输入失败不能中止无关输入。
- 汇总输出列出成功、警告和失败。

## 验收标准

- CLI 和 GUI 调用同一套核心 API。
- 扫描 JSON schema 有文档说明。
- 批量扫描具备确定性，并可按文件粒度恢复。
- 破坏性 CLI 路径通过 dry-run 测试覆盖。

## 开放问题

- CLI 应作为独立二进制发布，还是作为 Tauri sidecar？
- JSON schema 是否应独立于应用版本单独版本化？
