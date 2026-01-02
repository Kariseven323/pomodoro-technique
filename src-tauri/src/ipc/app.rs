//! 应用级 IPC 命令：快照、数据目录等。

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
    // 用 warn 级别保证在用户将日志级别设为 warn 时仍可看到“按钮点击是否触发命令”的关键线索。
    tracing::warn!(
        target: "system",
        "请求打开数据根目录：dir={}",
        store_dir.to_string_lossy()
    );

    std::fs::create_dir_all(&store_dir)
        .map_err(|e| AppError::Invariant(format!("创建根目录失败：{e}")))?;

    let path = store_dir.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        // Windows 上优先用 explorer.exe 打开目录（最符合用户预期）。
        match std::process::Command::new("explorer.exe")
            .arg(&store_dir)
            .spawn()
        {
            Ok(child) => {
                tracing::warn!(
                    target: "system",
                    "已启动 explorer.exe 打开目录：pid={}",
                    child.id()
                );
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    target: "system",
                    "explorer.exe 启动失败，将尝试 opener：path={} err={}",
                    path,
                    e
                );
            }
        }
    }

    // 非 Windows（或 Windows explorer 回退）：使用 opener 走系统默认打开逻辑。
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
    tracing::warn!(target: "system", "已通过 opener 请求打开目录：path={}", path);
    Ok(())
}
