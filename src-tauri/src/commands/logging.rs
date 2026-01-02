//! 日志相关命令：打开日志目录、前端诊断日志桥接等。

#[cfg(not(windows))]
use tauri_plugin_opener::OpenerExt as _;

use crate::errors::{AppError, AppResult};
use crate::logging;

use super::common::to_ipc_result;

/// 打开日志目录（文件管理器）。
#[tauri::command]
pub fn open_log_dir(app: tauri::AppHandle) -> Result<bool, String> {
    to_ipc_result(open_log_dir_impl(&app))
}

/// 前端日志桥接：将前端诊断信息写入后端文件日志（用于定位 WebView/布局问题）。
#[tauri::command]
pub fn frontend_log(level: String, message: String) -> Result<bool, String> {
    to_ipc_result(frontend_log_impl(&level, &message))
}

/// 前端日志桥接的内部实现：按 level 写入 tracing。
fn frontend_log_impl(level: &str, message: &str) -> AppResult<bool> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation(
            "仅开发环境可使用前端诊断日志（frontend_log）".to_string(),
        ));
    }

    let lvl = level.trim().to_lowercase();
    match lvl.as_str() {
        "debug" => tracing::debug!(target: "frontend", "{message}"),
        "warn" | "warning" => tracing::warn!(target: "frontend", "{message}"),
        "error" => tracing::error!(target: "frontend", "{message}"),
        _ => tracing::info!(target: "frontend", "{message}"),
    }
    Ok(true)
}

/// 打开日志目录的内部实现。
fn open_log_dir_impl(app: &tauri::AppHandle) -> AppResult<bool> {
    let dir = logging::log_dir(app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Invariant(format!("创建日志目录失败：{e}")))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| AppError::Invariant(format!("打开日志目录失败：{e}")))?;
        return Ok(true);
    }

    #[cfg(not(windows))]
    {
        app.opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| AppError::Invariant(format!("打开日志目录失败：{e}")))?;
        Ok(true)
    }
}
