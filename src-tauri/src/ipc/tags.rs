//! 标签相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::commands::common::to_ipc_result;
use crate::commands::tags::{add_tag_impl, delete_tag_impl, rename_tag_impl, set_current_tag_impl};
use crate::commands::types::AppSnapshot;
use crate::state::AppState;

/// 设置当前任务标签（用于下一条工作记录）。
#[tauri::command]
pub fn set_current_tag(
    state: tauri::State<'_, AppState>,
    tag: String,
) -> Result<AppSnapshot, String> {
    to_ipc_result(set_current_tag_impl(&*state, tag))
}

/// 新增一个标签（去重、去空白）。
#[tauri::command]
pub fn add_tag(state: tauri::State<'_, AppState>, tag: String) -> Result<Vec<String>, String> {
    to_ipc_result(add_tag_impl(&*state, tag))
}

/// 重命名标签（同时更新历史记录中的标签字段）。
#[tauri::command]
pub fn rename_tag(
    state: tauri::State<'_, AppState>,
    from: String,
    to: String,
) -> Result<AppSnapshot, String> {
    to_ipc_result(rename_tag_impl(&*state, from, to))
}

/// 删除标签（同时清空历史记录中的该标签）。
#[tauri::command]
pub fn delete_tag(state: tauri::State<'_, AppState>, tag: String) -> Result<AppSnapshot, String> {
    to_ipc_result(delete_tag_impl(&*state, tag))
}
