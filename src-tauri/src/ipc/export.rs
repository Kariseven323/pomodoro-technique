//! 导出相关 IPC 命令：负责系统对话框交互，导出逻辑复用可测试实现。

use tauri_plugin_dialog::DialogExt as _;

use crate::commands::common::to_ipc_result;
use crate::commands::export::{default_export_file_name, export_history_to_path};
use crate::commands::types::ExportRequest;
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

/// 导出历史记录：弹出保存对话框并写入 CSV/JSON，返回保存的文件路径。
#[tauri::command]
pub async fn export_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<String, String> {
    to_ipc_result(export_history_ipc_impl(&app, &*state, request))
}

/// IPC 内部实现：校验范围、弹出保存对话框、并调用可测试的导出写入逻辑。
fn export_history_ipc_impl(
    app: &tauri::AppHandle,
    state: &AppState,
    request: ExportRequest,
) -> AppResult<String> {
    tracing::warn!(
        target: "storage",
        "export_history 开始：from={} to={} format={:?} fields={}",
        request.range.from,
        request.range.to,
        request.format,
        request.fields.len()
    );
    let default_name = default_export_file_name(&request.range, request.format.clone());

    let Some(path) = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file()
    else {
        tracing::warn!(target: "storage", "export_history 已取消（用户关闭保存对话框）");
        return Err(AppError::Validation("已取消导出".to_string()));
    };

    let path = path
        .into_path()
        .map_err(|_| AppError::Invariant("导出路径解析失败".to_string()))?;

    export_history_to_path(state, &request, &path)?;

    tracing::warn!(
        target: "storage",
        "export_history 成功：path={}",
        path.to_string_lossy()
    );
    Ok(path.to_string_lossy().to_string())
}
