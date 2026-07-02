# RFC 0003：跨平台磁盘枚举

状态：通过
创建日期：2026-07-02
负责人：Beck

## 摘要

在 macOS、Linux 和 Windows 上一致地实现磁盘和镜像枚举，并包含分析与安全决策所需的元数据。

## 目标

- 列出物理磁盘、可移动磁盘、虚拟磁盘和镜像。
- 包含大小、显示名称、平台 ID、协议、挂载状态和访问能力。
- 识别权限问题，而不是把权限不足表现为空列表。
- 在所有支持平台上提供可预测的 UI 行为。

## 平台方案

| 平台 | 主要来源 | 备用方案 |
|------|----------|----------|
| macOS | `diskutil list -plist`、`diskutil info -plist` | `/dev/disk*` 探测 |
| Linux | `lsblk --json --bytes --output ...` | `/sys/block` 和 `blkid` |
| Windows | PowerShell `Get-Disk`、`Get-Partition`、`Get-Volume` | WMI / Win32 API |

## 数据契约

每个列表项应包含：

- 标准化 ID；
- 显示路径；
- 原始读取路径；
- 字节大小；
- 设备类型；
- 总线/协议；
- 可移动/外置标记；
- 挂载状态；
- 可读/可写状态；
- 不可用时的权限消息。

## UI 要求

- 空列表和权限不足状态必须有明显区别。
- 不可读磁盘仍可选择用于诊断，但破坏性操作保持阻止。
- 镜像应和物理磁盘明确区分。

## 验收标准

- macOS、Linux 和 Windows 的磁盘枚举可通过 CI 或 fixture 测试验证。
- 权限不足磁盘显示可操作的提示。
- 当平台提供元数据时，可以区分 USB/外置磁盘和内置磁盘。
- 扫描器可以在所有平台打开镜像文件。

## 风险

- Windows 原始磁盘访问可能需要管理员权限。
- Linux 设备命名在 SATA、NVMe、loop 和可移动介质之间差异较大。
- macOS 的 APFS synthesized container 容易混淆物理磁盘和卷模型。
