# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Windows 番茄钟桌面应用，专注模式下自动终止黑名单进程。基于 Tauri 2 + SvelteKit 5 + Tailwind CSS 4 构建。

## 常用命令

```bash
# 开发（前端 + Tauri 同时启动）
bun run tauri dev

# 仅前端开发
bun run dev

# 类型检查
bun run check

# 构建生产版本
bun run tauri build

# Rust 格式化
cd src-tauri && cargo fmt

# Rust 检查
cd src-tauri && cargo clippy
```

## 架构

### 前端 (src/)

- `routes/+page.svelte` - 主界面（计时器、标签选择、今日统计）
- `lib/components/` - 模态框组件（设置、黑名单管理）
- `lib/tauriApi.ts` - Tauri 命令调用封装
- `lib/types.ts` - 前后端共享类型定义

### 后端 (src-tauri/src/)

- `lib.rs` - 应用入口，插件注册，命令绑定
- `commands.rs` - Tauri 命令实现
- `timer.rs` - 计时器状态机（work/shortBreak/longBreak 阶段切换）
- `processes.rs` - 进程枚举与终止（Windows API）
- `state.rs` - 全局状态管理（AppState）
- `app_data.rs` - 持久化数据结构
- `tray.rs` - 系统托盘菜单

### 数据流

1. 前端通过 `invoke()` 调用 Rust 命令
2. 后端通过 `emit()` 推送事件（`timer-tick`、`kill-summary`）
3. 持久化使用 `tauri-plugin-store`，存储于 `%APPDATA%`

## 代码规范

- 所有方法和函数必须添加中文注释
- 使用中文回复
