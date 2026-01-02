//! 应用级 IPC 命令：快照、数据目录等。

#[cfg(not(windows))]
use tauri_plugin_opener::OpenerExt as _;

use crate::app_paths;
use crate::commands::app::get_store_paths_impl;
use crate::commands::common::to_ipc_result;
use crate::commands::types::{AppSnapshot, StorePaths};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

/// 获取应用完整快照（用于前端首屏渲染与恢复）。
#[tauri::command]
pub fn get_app_snapshot(state: tauri::State<'_, AppState>) -> Result<AppSnapshot, String> {
    to_ipc_result(Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    }))
}

/// 获取应用数据根目录路径（统一入口，便于用户一处查看与备份）。
#[tauri::command]
pub fn get_store_paths(app: tauri::AppHandle) -> Result<StorePaths, String> {
    to_ipc_result(get_store_paths_impl(&app))
}

/// 打开应用数据根目录（文件管理器，统一入口）。
#[tauri::command]
pub fn open_store_dir(app: tauri::AppHandle) -> Result<(), String> {
    to_ipc_result(open_store_dir_impl(&app))
}

/// 打开应用数据根目录的内部实现：确保目录存在后再请求系统打开（统一入口）。
fn open_store_dir_impl(app: &tauri::AppHandle) -> AppResult<()> {
    let store_dir = app_paths::app_root_dir(app)?;

    std::fs::create_dir_all(&store_dir)
        .map_err(|e| AppError::Invariant(format!("创建根目录失败：{e}")))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&store_dir)
            .spawn()
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        app.opener()
            .open_path(store_dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
        Ok(())
    }
}

