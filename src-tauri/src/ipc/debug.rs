//! 调试相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::commands::common::to_ipc_result;
use crate::commands::debug::{debug_clear_history_impl, debug_generate_history_impl};
use crate::errors::AppResult;
use crate::state::AppState;

/// 开发者命令：一键生成测试历史数据并写入 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_generate_history(state: tauri::State<'_, AppState>, days: u32) -> Result<u32, String> {
    to_ipc_result(debug_generate_history_ipc_impl(&*state, days))
}

/// 开发者命令：清空 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_clear_history(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    to_ipc_result(debug_clear_history_ipc_impl(&*state))
}

/// IPC 内部实现：复用 `commands::debug` 的可测试实现。
fn debug_generate_history_ipc_impl(state: &AppState, days: u32) -> AppResult<u32> {
    debug_generate_history_impl(state, days)
}

/// IPC 内部实现：复用 `commands::debug` 的可测试实现。
fn debug_clear_history_ipc_impl(state: &AppState) -> AppResult<bool> {
    debug_clear_history_impl(state)
}
