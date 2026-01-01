<div align="center">

# 🍅 Pomodoro Technique

**专注力守护者 —— 让分心程序无处遁形**

[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-5.0-FF3E00?style=for-the-badge&logo=svelte&logoColor=white)](https://kit.svelte.dev/)
[![Rust](https://img.shields.io/badge/Rust-2026-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind-4.0-06B6D4?style=for-the-badge&logo=tailwindcss&logoColor=white)](https://tailwindcss.com/)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)

<img src="src-tauri/icons/icon.png" width="128" height="128" alt="Pomodoro Icon">

*一款为 Windows 打造的番茄钟桌面应用，在专注模式下自动终止干扰程序，让你的工作效率飙升 🚀*

[功能特性](#-功能特性) •
[快速开始](#-快速开始) •
[技术架构](#-技术架构) •
[开发指南](#-开发指南)

</div>

---

## ✨ 功能特性

<table>
<tr>
<td width="50%">

### 🎯 智能番茄钟
- **经典番茄工作法**：25 分钟工作 + 5 分钟短休息
- **自动阶段切换**：工作 → 短休息 → 长休息循环
- **灵活时间配置**：自定义各阶段时长
- **实时倒计时**：精确到秒的计时显示

</td>
<td width="50%">

### 🛡️ 进程守护
- **黑名单管理**：添加需要屏蔽的干扰程序
- **自动终止**：专注期间自动 kill 黑名单进程
- **智能锁定**：专注时黑名单只增不减
- **管理员提权**：一键提升权限终止顽固进程

</td>
</tr>
<tr>
<td width="50%">

### 🏷️ 任务标签
- **多标签支持**：为不同任务创建专属标签
- **快速切换**：一键切换当前工作标签
- **统计追踪**：按标签统计今日完成数

</td>
<td width="50%">

### 📊 数据统计
- **今日概览**：实时显示今日完成的番茄数
- **分类统计**：按标签查看完成情况
- **本地持久化**：数据安全存储于本地

</td>
</tr>
</table>

### 🎨 更多亮点

- 🌓 **深色模式**：自动适配系统主题，护眼又美观
- 🔔 **系统通知**：阶段切换时推送桌面通知
- 📌 **系统托盘**：最小化到托盘，静默守护
- ⚡ **极致性能**：Rust 后端 + 原生 WebView，内存占用 < 30MB
- 🎯 **零配置**：开箱即用，无需复杂设置

---

## 🚀 快速开始

### 系统要求

- Windows 10/11 (64-bit)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows 11 已内置)

### 安装方式

#### 方式一：下载安装包

前往 [Releases](../../releases) 页面下载最新版本的 `.msi` 或 `.exe` 安装包。

#### 方式二：从源码构建

```bash
# 克隆仓库
git clone https://github.com/Kariseven323/pomodoro-technique.git
cd pomodoro-technique

# 安装依赖
bun install

# 开发模式运行
bun run tauri dev

# 构建生产版本
bun run tauri build
```

---

## 🏗️ 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  SvelteKit  │  │  Tailwind   │  │    TypeScript       │  │
│  │     5.0     │  │  CSS 4.0    │  │       5.6           │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                     Tauri IPC Bridge                         │
│              invoke() ↓↑ emit() Event System                 │
├─────────────────────────────────────────────────────────────┤
│                        Backend (Rust)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Timer     │  │  Process    │  │    Windows API      │  │
│  │   Engine    │  │  Manager    │  │    Integration      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   State     │  │   Store     │  │    Tray & Notify    │  │
│  │   Machine   │  │  Plugin     │  │      System         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 核心模块

| 模块 | 路径 | 职责 |
|------|------|------|
| **Timer Engine** | `src-tauri/src/timer.rs` | 计时器状态机，管理 work/shortBreak/longBreak 阶段 |
| **Process Manager** | `src-tauri/src/processes.rs` | 进程枚举与终止，调用 Windows API |
| **State Manager** | `src-tauri/src/state.rs` | 全局状态管理 (AppState) |
| **Commands** | `src-tauri/src/commands.rs` | Tauri 命令实现，前后端桥接 |
| **Frontend** | `src/routes/+page.svelte` | 主界面 UI，计时器、标签、统计 |

---

## 🛠️ 开发指南

### 环境准备

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Bun
curl -fsSL https://bun.sh/install | bash

# 安装项目依赖
bun install
```

### 常用命令

```bash
# 开发模式（前端 + Tauri 热重载）
bun run tauri dev

# 仅前端开发
bun run dev

# TypeScript 类型检查
bun run check

# 构建生产版本
bun run tauri build

# Rust 代码格式化
cd src-tauri && cargo fmt

# Rust 代码检查
cd src-tauri && cargo clippy
```

### 项目结构

```
pomodoro-technique/
├── src/                      # 前端源码
│   ├── routes/
│   │   └── +page.svelte      # 主页面
│   └── lib/
│       ├── components/       # UI 组件
│       ├── tauriApi.ts       # Tauri API 封装
│       └── types.ts          # 类型定义
├── src-tauri/                # Rust 后端
│   ├── src/
│   │   ├── lib.rs            # 应用入口
│   │   ├── commands.rs       # Tauri 命令
│   │   ├── timer.rs          # 计时器逻辑
│   │   ├── processes.rs      # 进程管理
│   │   ├── state.rs          # 状态管理
│   │   └── tray.rs           # 系统托盘
│   └── Cargo.toml            # Rust 依赖
├── package.json              # 前端依赖
└── README.md
```

---

## 📝 使用说明

1. **启动应用** - 双击运行，应用将显示在系统托盘
2. **配置黑名单** - 点击「管理黑名单」添加需要屏蔽的程序
3. **选择标签** - 为当前任务选择或创建标签
4. **开始专注** - 点击「开始」进入 25 分钟专注模式
5. **自动守护** - 专注期间，黑名单程序将被自动终止

> 💡 **提示**：如果某些程序需要管理员权限才能终止，点击「以管理员身份重启」即可。

---

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 📄 开源协议

本项目基于 [MIT License](LICENSE) 开源。

---

## 🌟 Star History

<a href="https://star-history.com/#Kariseven323/pomodoro-technique&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=Kariseven323/pomodoro-technique&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=Kariseven323/pomodoro-technique&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=Kariseven323/pomodoro-technique&type=Date" />
 </picture>
</a>

---

<div align="center">

**如果这个项目对你有帮助，请给一个 ⭐ Star 支持一下！**

Made with ❤️ and 🍅

</div>
