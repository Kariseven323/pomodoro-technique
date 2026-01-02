//! 黑名单相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::app_data::BlacklistItem;
use crate::commands::blacklist::set_blacklist_impl;
use crate::commands::common::to_ipc_result;
use crate::errors::AppResult;
use crate::state::AppState;

/// 更新黑名单列表（专注期间会锁定，禁止移除）。
#[tauri::command]
pub fn set_blacklist(
    state: tauri::State<'_, AppState>,
    blacklist: Vec<BlacklistItem>,
) -> Result<Vec<BlacklistItem>, String> {
    to_ipc_result((|| -> AppResult<Vec<BlacklistItem>> {
        let out = set_blacklist_ipc_impl(&*state, blacklist)?;
        let _ = crate::tray::refresh_tray(&*state);
        Ok(out)
    })())
}

/// IPC 内部实现：复用 `commands::blacklist` 的可测试实现。
fn set_blacklist_ipc_impl(state: &AppState, blacklist: Vec<BlacklistItem>) -> AppResult<Vec<BlacklistItem>> {
    set_blacklist_impl(state, blacklist)
}
