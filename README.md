# Mini TestDisk

基于 [Rust](https://www.rust-lang.org/) + [Tauri 2](https://tauri.app/) 的跨平台桌面 GUI，复刻 [cgsecurity/testdisk](https://github.com/cgsecurity/testdisk) 的核心分区分析流程，用于验证技术可行性。

## 功能

- 枚举系统磁盘（macOS / Linux）
- 打开磁盘镜像文件（`.img`、`.iso`、`.dmg` 等）离线分析
- 自动识别 **MBR** / **GPT** 分区表
- GPT 头 CRC32 校验
- 文件系统签名检测：FAT、NTFS、exFAT、ext2/3/4、APFS、HFS/HFS+
- 深度扫描：在 1 MB 边界搜索丢失分区（简化版 Deeper Search）
- macOS 外置/USB 磁盘整盘格式化，可选择 APFS、ExFAT、FAT32、Mac OS Extended
- 标准化磁盘目标元数据：平台 ID、显示路径、读取路径、访问能力、安全分类
- 扫描结果可导出为 JSON 报告
- 前端支持明暗主题切换与中英文切换，面向实际磁盘恢复工作流

> 分区分析为只读流程。格式化是破坏性写盘操作，当前仅允许 macOS 外置/USB 整盘格式化，不包含分区写入修复与 PhotoRec 文件雕刻。

## 系统要求

| 依赖 | 版本 |
|------|------|
| Rust | ≥ 1.77（推荐通过 [rustup](https://rustup.rs/) 安装） |
| Node.js / npm | Node.js ≥ 20（用于 Vue 前端构建） |
| Tauri CLI | 2.x（`cargo install tauri-cli --locked`） |

### 平台额外依赖

**macOS**

- Xcode Command Line Tools：`xcode-select --install`

**Linux — Tauri 2**（Ubuntu 22.04+ / Debian 12+）

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev patchelf build-essential curl wget file libssl-dev pkg-config
```

**Linux — Tauri 1**（Ubuntu 20.04 等仅含 WebKitGTK 4.0 的系统）

项目会在 `make dev` / `make build` 时**自动检测**系统 WebKit 版本并选择 Tauri 1.x 或 2.x：

```bash
make tauri-version   # 查看检测结果（20.04 输出 1，22.04+ 输出 2）
```

Ubuntu 20.04 需安装：

```bash
sudo apt update
sudo apt install -y \
  pkg-config build-essential \
  libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.0-dev \
  libappindicator3-dev librsvg2-dev patchelf curl wget file libssl-dev
```

也可手动指定版本：

```bash
make dev-v1   # 强制 Tauri 1.x
make dev-v2   # 强制 Tauri 2.x（需 WebKitGTK 4.1）
```

**Windows**

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)（Windows 10/11 通常已内置）

## 快速开始

```bash
# 1. 克隆仓库
git clone <repo-url> testdisk && cd testdisk

# 2. 检查工具链
make check
npm install

# 若缺少 Tauri CLI：
make install-tauri-cli

# 3. 生成测试镜像（可选，无需 root）
make sample-image

# 4. 启动开发版 GUI
make dev
```

> **SSH 远程开发：** 若 `make dev` 报 `cannot open display`，说明当前 SSH 会话没有图形环境。请用 `ssh -X user@host` 重新登录（客户端需 X 服务器），或在本地桌面终端运行。无界面冒烟测试：`sudo apt install xvfb && make dev-xvfb`。

在 GUI 中：**打开磁盘镜像...** → 选择 `testdata/sample-mbr.img` → **分析分区**。

## 构建

### 当前平台（推荐）

```bash
make build          # Release 安装包
make build-debug    # Debug 版本
make test           # 运行单元测试
make test-core      # 仅核心库测试
make clean          # 清理构建产物
make info           # 查看输出路径
make help           # 所有 Make 目标
```

### 跨平台构建说明

Tauri 原生 GUI 需在**目标操作系统上**构建安装包：

| 目标平台 | 命令 | 输出格式 |
|----------|------|----------|
| macOS | `make build-macos` | `.app`、`.dmg` |
| Linux | `make build-linux` | `.deb`、`.AppImage` |
| Windows | `make build-windows` | `.exe`、`.msi` |

构建完成后，产物位于：

```
target/release/bundle/
├── macos/          # Mini TestDisk.app
├── dmg/            # Mini TestDisk_*.dmg
├── deb/            # *.deb
├── appimage/       # *.AppImage
├── msi/            # *.msi
└── nsis/           # *.exe
```

在 CI 中可分别为三个 OS 各跑一条 `make build-*` 流水线，实现跨平台发布。

### GitHub Actions 自动发布

仓库已配置 CI/CD 工作流：

| 工作流 | 触发条件 | 说明 |
|--------|----------|------|
| [CI](.github/workflows/ci.yml) | push / PR → `main` | 单元测试 + Linux 编译验证 |
| [Release](.github/workflows/release.yml) | 推送 tag `v*` 或手动触发 | 四端并行打包并发布到 GitHub Releases |

**发布新版本：**

```bash
# 1. 更新 src-tauri/tauri.conf.json 中的 version
# 2. 提交并打 tag
git tag v0.1.0
git push origin v0.1.0
```

或在 GitHub 网页：**Actions → Release → Run workflow**，填写 tag（如 `v0.1.0`）。

Release 工作流会在以下 Runner 上并行构建：

- macOS Apple Silicon (`aarch64-apple-darwin`) → `.dmg`
- macOS Intel (`x86_64-apple-darwin`) → `.dmg`
- Ubuntu 22.04 → `.deb`、`.AppImage`
- Windows → `.msi`、`.exe`

**首次使用前**，在仓库 **Settings → Actions → General → Workflow permissions** 中开启 **Read and write permissions**，否则上传 Release 会失败。

```mermaid
flowchart LR
  tag["git tag v0.1.0"] --> release["Release workflow"]
  release --> mac_arm["macOS arm64"]
  release --> mac_x64["macOS x64"]
  release --> linux["Linux"]
  release --> win["Windows"]
  mac_arm --> gh["GitHub Releases"]
  mac_x64 --> gh
  linux --> gh
  win --> gh
```

## 项目结构

```
testdisk/
├── .github/workflows/       # CI / Release 工作流
│   ├── ci.yml
│   └── release.yml
├── Makefile                 # 跨平台构建入口
├── README.md
├── Cargo.toml               # Workspace 根
├── crates/
│   └── testdisk-core/       # 核心库（分区表 + 文件系统检测）
│       ├── domain.rs
│       ├── disk.rs
│       ├── mbr.rs
│       ├── gpt.rs
│       ├── fs_detect.rs
│       ├── report.rs
│       └── scanner.rs
│   └── testdisk-platform/   # 平台适配层（枚举 / 镜像 / 格式化入口）
│       ├── adapter.rs
│       ├── disk.rs
│       └── formatter.rs
├── src-tauri/               # Tauri 2 后端（Ubuntu 22.04+ / WebKitGTK 4.1）
│   ├── src/lib.rs
│   └── tauri.conf.json
├── src-tauri-v1/            # Tauri 1 后端（Ubuntu 20.04 / WebKitGTK 4.0）
│   ├── src/lib.rs
│   └── tauri.conf.json
├── scripts/
│   ├── detect-tauri-version.sh
│   └── run-tauri.sh
├── package.json             # Vue / Vite 前端依赖与脚本
├── vite.config.js           # Vite 构建配置
├── ui/                      # Vue 前端
│   ├── index.html
│   ├── src/
│   │   ├── App.vue
│   │   ├── i18n.js
│   │   └── main.js
│   ├── styles.css
└── testdata/
    └── sample-mbr.img       # make sample-image 生成
```

## 架构

```
┌─────────────────────────────────────┐
│  ui/          Tauri WebView 前端     │
│  App.vue + i18n + theme state        │
└──────────────┬──────────────────────┘
               │ invoke (IPC)
┌──────────────▼──────────────────────┐
│  src-tauri/   Tauri 命令层           │
│  get_disks / scan_disk_path / ...   │
│  依赖平台适配层，不直接拼平台命令   │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  testdisk-platform/ 平台适配层      │
│  磁盘枚举 · 镜像打开 · 格式化入口   │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  testdisk-core/  纯 Rust 核心库      │
│  domain · platform · scanner · report│
│  MBR · GPT · FS 检测 · 深度扫描      │
└──────────────┬──────────────────────┘
               │ 只读
┌──────────────▼──────────────────────┐
│  块设备 /dev/disk*  或  磁盘镜像      │
└─────────────────────────────────────┘
```

## 权限说明

- **磁盘镜像**：普通用户即可读写分析。
- **物理磁盘**（如 `/dev/disk0`）：macOS / Linux 通常需要 root 权限。macOS 对 `/dev/disk*` 原始块设备不会弹出普通应用授权窗口；如果列表中磁盘显示为“需权限”，请用管理员权限启动开发版，或先制作磁盘镜像再分析：

  ```bash
  # macOS 示例（谨慎操作，只读打开）
  sudo make dev
  ```

## 格式化说明

格式化功能当前只支持 macOS 外置/USB 整盘格式化。后端会通过 `diskutil info -plist` 校验目标磁盘，拒绝格式化内置磁盘、未知磁盘和分区设备（如 `disk4s1`）。

支持的文件系统：

| 文件系统 | 适用场景 |
|----------|----------|
| APFS | macOS 当前主流格式 |
| ExFAT | macOS / Windows / Linux 跨平台 U 盘 |
| MS-DOS FAT32 | 老设备兼容，单文件 4GB 限制 |
| Mac OS Extended (Journaled) | 老 macOS / HFS+ 设备兼容 |

执行前必须在界面中输入目标磁盘路径进行二次确认。格式化会删除目标磁盘全部数据。

## RFC 路线图

项目路线图位于 [`RFC/`](RFC/)，中文版本位于 [`RFC/zh-CN/`](RFC/zh-CN/)。

当前 `feat/rfc-0001-roadmap` 分支已落地 RFC-0001 Phase 0 的基础能力：

- 扩展后端 `DiskInfo`，增加跨平台目标元数据和安全分类字段。
- 增加 `ScanReport` JSON schema，并支持从 GUI 导出扫描报告。
- 前端增加“目标信息”面板，展示平台 ID、显示路径、读取路径、协议、访问能力和安全分类。
- 前端增加主题和语言切换，所有可见文案开始通过统一 i18n 映射管理。
- 核心 Rust 代码按 domain / platform / scanner / report 分层拆分，避免继续把共享模型和平台逻辑堆在单一文件中。

## 与原版 TestDisk 对比

| 能力 | 原版 TestDisk | Mini TestDisk |
|------|---------------|---------------|
| MBR / GPT 解析 | ✅ | ✅ |
| 文件系统签名检测 | 30+ 种 | 8 种（常见） |
| 丢失分区深度搜索 | ✅ 完整 | ✅ 简化 |
| 分区表修复写入 | ✅ | ❌ 未实现 |
| PhotoRec 文件雕刻 | ✅ | ❌ 未实现 |
| 跨平台 GUI | ncurses TUI | Tauri 原生窗口 |

## 许可证

核心思路参考 [cgsecurity/testdisk](https://github.com/cgsecurity/testdisk)（GPL-2.0）。本项目代码请根据实际需要自行选择许可证。

## 参考

- [TestDisk 官方文档](https://www.cgsecurity.org/wiki/TestDisk)
- [TestDisk GitHub](https://github.com/cgsecurity/testdisk)
- [Tauri 文档](https://tauri.app/start/)
