//! 前端可调用的 Tauri 命令集合（按领域拆分，避免单文件过大）。

pub mod analysis;
pub mod app;
pub mod blacklist;
pub(crate) mod common;
pub mod debug;
pub mod export;
pub mod history;
pub mod logging;
pub mod processes;
pub mod settings;
pub mod tags;
pub mod templates;
pub mod timer;
mod state_like;
mod validation;

pub mod types;
