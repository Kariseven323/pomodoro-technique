//! 应用路径解析：提供“统一入口”的根目录、数据目录与文件路径工具。

use std::path::PathBuf;

#[cfg(not(windows))]
use tauri::Manager as _;

use crate::app_data::STORE_FILE_NAME;
use crate::errors::{AppError, AppResult};

/// 获取统一入口的根目录。
///
/// - Windows：`%APPDATA%/pomodoro-technique/`
/// - 其他平台：`app_data_dir/`（Tauri 标准应用数据目录）
pub fn app_root_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    #[cfg(windows)]
    {
        let _ = app;
        let appdata = std::env::var("APPDATA")
            .map_err(|_| AppError::Invariant("无法读取环境变量 APPDATA".to_string()))?;
        return Ok(PathBuf::from(appdata).join("pomodoro-technique"));
    }

    #[cfg(not(windows))]
    {
        app.path()
            .app_data_dir()
            .map_err(|_| AppError::Invariant("无法解析应用数据目录（app_data_dir）".to_string()))
    }
}

/// 获取应用数据目录（位于统一入口根目录下的 `data/`）。
pub fn app_data_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(app_root_dir(app)?.join("data"))
}

/// 获取 store 文件的最终落盘路径（完整路径）。
pub fn store_file_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join(STORE_FILE_NAME))
}

/// 获取应用日志目录（位于统一入口根目录下的 `logs/`）。
pub fn app_log_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(app_root_dir(app)?.join("logs"))
}
