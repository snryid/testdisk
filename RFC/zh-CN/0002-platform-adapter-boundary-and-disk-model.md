# RFC 0002：平台适配器边界和磁盘模型

状态：通过
创建日期：2026-07-02
负责人：Beck

## 摘要

定义稳定的磁盘模型和平台适配器边界，让 Mini TestDisk 能支持 macOS、Linux 和 Windows，同时避免操作系统相关逻辑散落在核心扫描器和 UI 中。

## 目标

- 将共享磁盘分析逻辑与平台特定设备访问分离。
- 统一建模磁盘、分区、卷和镜像。
- 提供所有破坏性操作都必须使用的安全分类。
- 让平台命令输出解析可以通过 fixture 测试。

## 非目标

- 本 RFC 不完整实现所有平台的磁盘枚举。
- 不增加新的写入操作。
- 不替换当前扫描算法。

## 建议模型

核心类型应表示：

- `DiskTarget`：完整物理磁盘、虚拟磁盘或镜像文件。
- `PartitionTarget`：带起止 LBA 和字节范围的分区表条目。
- `VolumeTarget`：可挂载文件系统实例。
- `TargetSafety`：系统盘、内置盘、外置盘、可移动盘、镜像、未知。
- `AccessCapability`：可读、可写、需要提权、设备忙、不支持。

## 适配器边界

引入平台适配器 trait，包含：

- 列出磁盘；
- 将用户选择路径解析为标准目标；
- 读取磁盘元数据；
- 检查挂载状态；
- 进行安全分类；
- 生成平台特定操作计划。

`testdisk-core` 保留解析器和扫描逻辑。平台命令执行应位于纯解析逻辑之外。

## 跨平台要求

| 范围 | macOS | Linux | Windows |
|------|-------|-------|---------|
| 磁盘身份 | `diskN`、`/dev/rdiskN` | `/dev/sdX`、`/dev/nvmeXnY` | `PhysicalDriveN` |
| 元数据 | `diskutil info -plist` | `lsblk --json`、`/sys/block` | WMI / PowerShell |
| 安全分类 | internal/removable/protocol | removable/rotational/model | 总线类型、启动/系统标记 |

## 验收标准

- 前后端对磁盘和镜像目标使用同一共享模型。
- 平台特定标识被保留，但不是唯一身份来源。
- 单元测试覆盖 macOS、Linux 和 Windows 示例目标的规范化。
- 安全分类由后端生成，前端不能自行推断。

## 开放问题

- 平台适配器是否应立即放入 `crates/testdisk-platform`，还是先放在 `src-tauri`？
- 命令执行从一开始就使用异步任务运行器，还是先使用同步进程调用？
