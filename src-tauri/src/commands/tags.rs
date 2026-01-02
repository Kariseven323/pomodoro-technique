//! 标签相关命令：设置当前标签、管理标签列表。

use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;
use super::types::AppSnapshot;

/// 设置当前番茄的任务标签；若为新标签则追加到 tags 并持久化。
#[tauri::command]
pub fn set_current_tag(
    state: tauri::State<'_, AppState>,
    tag: String,
) -> Result<AppSnapshot, String> {
    to_ipc_result(set_current_tag_impl(&state, tag))
}

/// 设置当前标签的内部实现（便于统一错误处理）。
fn set_current_tag_impl(state: &AppState, tag: String) -> AppResult<AppSnapshot> {
    let clock = crate::timer::SystemClock;
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }

    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.set_current_tag(tag.clone(), &clock);
            if !data.tags.iter().any(|t| t == &tag) {
                data.tags.push(tag);
            }
            Ok(())
        },
        true,
    )?;

    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 新增一个标签到历史标签列表（持久化）。
#[tauri::command]
pub fn add_tag(state: tauri::State<'_, AppState>, tag: String) -> Result<Vec<String>, String> {
    to_ipc_result(add_tag_impl(&state, tag))
}

/// 新增标签的内部实现（便于统一错误处理）。
fn add_tag_impl(state: &AppState, tag: String) -> AppResult<Vec<String>> {
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }

    state.update_data(|data| {
        if !data.tags.iter().any(|t| t == &tag) {
            data.tags.push(tag);
        }
        Ok(())
    })?;

    Ok(state.data_snapshot().tags)
}
