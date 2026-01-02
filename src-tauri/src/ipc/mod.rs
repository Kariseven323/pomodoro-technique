//! IPC 命令入口：集中放置 `#[tauri::command]` 包装函数（排除 UI 口径的覆盖率统计）。

pub mod analysis;
pub mod app;
pub mod blacklist;
pub mod debug;
pub mod export;
pub mod history;
pub mod logging;
pub mod processes;
pub mod settings;
pub mod tags;
pub mod templates;
pub mod timer;
pub mod window;

