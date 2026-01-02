//! 设置相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::app_data::Settings;
use crate::commands::common::to_ipc_result;
use crate::commands::settings::{set_goals_impl, update_settings_impl};
use crate::commands::types::AppSnapshot;
use crate::errors::AppResult;
use crate::state::AppState;

/// 更新设置（带范围校验），并在必要时重置当前阶段的剩余时间。
#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<AppSnapshot, String> {
    to_ipc_result((|| -> AppResult<AppSnapshot> {
        let out = update_settings_ipc_impl(&*state, settings)?;
        let _ = crate::tray::refresh_tray(&*state);
        Ok(out)
    })())
}

/// 设置每日/每周目标（0 表示不设目标），并持久化到 settings。
#[tauri::command]
pub fn set_goals(
    state: tauri::State<'_, AppState>,
    daily: u32,
    weekly: u32,
) -> Result<Settings, String> {
    to_ipc_result(set_goals_ipc_impl(&*state, daily, weekly))
}

/// IPC 内部实现：复用 `commands::settings` 的可测试实现。
fn update_settings_ipc_impl(state: &AppState, settings: Settings) -> AppResult<AppSnapshot> {
    update_settings_impl(state, settings)
}

/// IPC 内部实现：复用 `commands::settings` 的可测试实现。
fn set_goals_ipc_impl(state: &AppState, daily: u32, weekly: u32) -> AppResult<Settings> {
    set_goals_impl(state, daily, weekly)
}
