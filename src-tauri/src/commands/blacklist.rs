//! 黑名单相关命令：设置黑名单、专注期锁定校验等。

use crate::app_data::BlacklistItem;
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;
use super::validation::{normalize_name, validate_blacklist_items};

/// 设置黑名单（专注期内仅允许“新增”，不允许移除）。
#[tauri::command]
pub fn set_blacklist(
    state: tauri::State<'_, AppState>,
    blacklist: Vec<BlacklistItem>,
) -> Result<Vec<BlacklistItem>, String> {
    to_ipc_result(set_blacklist_impl(&state, blacklist))
}

/// 设置黑名单的内部实现（便于统一错误处理）。
fn set_blacklist_impl(
    state: &AppState,
    blacklist: Vec<BlacklistItem>,
) -> AppResult<Vec<BlacklistItem>> {
    validate_blacklist_items(&blacklist)?;

    let (added_names, should_kill_added) = state.update_data_and_timer(
        |data, timer_runtime| {
            let locked = timer_runtime.blacklist_locked();

            if locked {
                let old_names: std::collections::BTreeSet<String> = data
                    .blacklist
                    .iter()
                    .map(|b| normalize_name(&b.name))
                    .collect();
                let new_names: std::collections::BTreeSet<String> =
                    blacklist.iter().map(|b| normalize_name(&b.name)).collect();

                if !old_names.is_subset(&new_names) {
                    return Err(AppError::BlacklistLocked);
                }
            }

            let old_names: std::collections::BTreeSet<String> = data
                .blacklist
                .iter()
                .map(|b| normalize_name(&b.name))
                .collect();

            let added: Vec<String> = blacklist
                .iter()
                .filter(|b| !old_names.contains(&normalize_name(&b.name)))
                .map(|b| b.name.clone())
                .collect();

            data.blacklist = blacklist.clone();

            // PRD：番茄周期内可动态添加并立即终止。
            let should_kill = locked && !added.is_empty();

            Ok((added, should_kill))
        },
        true,
    )?;

    if should_kill_added {
        tracing::info!(target: "blacklist", "专注期新增黑名单条目，立即尝试终止：{:?}", added_names);
        let payload = crate::processes::kill_names_best_effort(&added_names);
        let _ = state.emit_kill_result(payload);
    }

    Ok(state.data_snapshot().blacklist)
}
