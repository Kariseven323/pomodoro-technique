//! 计时器相关命令：开始/暂停/重置/跳过（供前端与托盘复用）。

use crate::app_data::Phase;
use crate::errors::AppResult;
use crate::state::AppState;
use crate::timer::{TimerSnapshot, TodayStats};

use super::common::to_ipc_result;

/// 开始计时（若处于工作阶段首次开始，则终止黑名单进程）。
#[tauri::command]
pub fn timer_start(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_start_impl(&state))
}

/// 托盘复用：开始计时的内部实现（不暴露给前端）。
pub fn timer_start_inner(state: &AppState) -> AppResult<()> {
    let (names_to_kill, should_kill) = state.update_data_and_timer(
        |data, timer_runtime| {
            let should_kill = timer_runtime.phase == Phase::Work
                && !timer_runtime.blacklist_locked()
                && !timer_runtime.is_running;
            let names: Vec<String> = data.blacklist.iter().map(|b| b.name.clone()).collect();
            timer_runtime.start(&data.settings);
            Ok((names, should_kill))
        },
        false,
    )?;

    tracing::info!(
        target: "timer",
        "开始计时：phase={:?} tag={} remaining={}s",
        state.timer_snapshot().phase,
        state.timer_snapshot().current_tag,
        state.timer_snapshot().remaining_seconds
    );

    if should_kill {
        tracing::info!(target: "blacklist", "工作阶段首次开始，尝试终止黑名单进程：{:?}", names_to_kill);
        let payload = crate::processes::kill_names_best_effort(&names_to_kill);
        let _ = state.emit_kill_result(payload);
    }

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 开始计时的 IPC 入口实现：先执行内部逻辑，再返回快照。
fn timer_start_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_start_inner(state)?;
    Ok(state.timer_snapshot())
}

/// 暂停计时。
#[tauri::command]
pub fn timer_pause(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_pause_impl(&state))
}

/// 托盘复用：暂停计时的内部实现（不暴露给前端）。
pub fn timer_pause_inner(state: &AppState) -> AppResult<()> {
    state.update_timer(|timer_runtime, _data| {
        timer_runtime.pause();
        Ok(())
    })?;

    tracing::info!(
        target: "timer",
        "暂停计时：phase={:?} remaining={}s",
        state.timer_snapshot().phase,
        state.timer_snapshot().remaining_seconds
    );

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 暂停计时的 IPC 入口实现：先执行内部逻辑，再返回快照。
fn timer_pause_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_pause_inner(state)?;
    Ok(state.timer_snapshot())
}

/// 重置计时（回到工作阶段初始状态）。
#[tauri::command]
pub fn timer_reset(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_reset_impl(&state))
}

/// 重置计时的内部实现（便于统一错误处理）。
fn timer_reset_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.reset(&data.settings);
            Ok(())
        },
        false,
    )?;

    tracing::info!(target: "timer", "重置计时器：回到工作阶段");
    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(state.timer_snapshot())
}

/// 跳过当前阶段（不写入历史）。
#[tauri::command]
pub fn timer_skip(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_skip_impl(&state))
}

/// 跳过阶段的内部实现（便于统一错误处理）。
fn timer_skip_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            let completed_today = TodayStats::from_app_data(data).total;
            timer_runtime.skip(&data.settings, completed_today);
            Ok(())
        },
        false,
    )?;

    tracing::info!(target: "timer", "跳过阶段：phase={:?}", state.timer_snapshot().phase);
    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(state.timer_snapshot())
}
