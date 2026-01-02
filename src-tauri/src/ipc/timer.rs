//! 计时器相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::commands::common::to_ipc_result;
use crate::commands::timer::{
    timer_pause_impl, timer_reset_impl, timer_skip_impl, timer_start_impl,
};
use crate::errors::AppResult;
use crate::state::AppState;
use crate::timer::TimerSnapshot;

/// 启动计时器（从当前阶段开始倒计时）。
#[tauri::command]
pub fn timer_start(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result((|| -> AppResult<TimerSnapshot> {
        let before = state.timer_snapshot();
        let snapshot = timer_start_ipc_impl(&*state)?;
        if before.phase == crate::app_data::Phase::Work
            && !before.blacklist_locked
            && !before.is_running
        {
            let _ = state.on_work_started_for_combo();
        }
        let _ = state.sync_audio_with_timer();
        let _ = crate::tray::refresh_tray(&*state);
        Ok(snapshot)
    })())
}

/// 暂停计时器（不重置剩余时间）。
#[tauri::command]
pub fn timer_pause(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result((|| -> AppResult<TimerSnapshot> {
        let snapshot = timer_pause_ipc_impl(&*state)?;
        let _ = state.sync_audio_with_timer();
        let _ = crate::tray::refresh_tray(&*state);
        Ok(snapshot)
    })())
}

/// 重置计时器（回到当前阶段默认时长，停止运行）。
#[tauri::command]
pub fn timer_reset(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result((|| -> AppResult<TimerSnapshot> {
        let before = state.timer_snapshot();
        let snapshot = timer_reset_ipc_impl(&*state)?;
        if before.phase == crate::app_data::Phase::Work && before.blacklist_locked {
            let _ = state.on_interrupted_for_combo();
        }
        let _ = state.sync_audio_with_timer();
        let _ = crate::tray::refresh_tray(&*state);
        Ok(snapshot)
    })())
}

/// 跳过当前阶段（不会写入历史；切换到下一阶段并停止）。
#[tauri::command]
pub fn timer_skip(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result((|| -> AppResult<TimerSnapshot> {
        let before = state.timer_snapshot();
        let snapshot = timer_skip_ipc_impl(&*state)?;
        if before.phase == crate::app_data::Phase::Work && before.blacklist_locked {
            let _ = state.on_interrupted_for_combo();
        }
        let _ = state.sync_audio_with_timer();
        let _ = crate::tray::refresh_tray(&*state);
        Ok(snapshot)
    })())
}

/// IPC 内部实现：复用 `commands::timer` 的可测试实现。
fn timer_start_ipc_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_start_impl(state)
}

/// IPC 内部实现：复用 `commands::timer` 的可测试实现。
fn timer_pause_ipc_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_pause_impl(state)
}

/// IPC 内部实现：复用 `commands::timer` 的可测试实现。
fn timer_reset_ipc_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_reset_impl(state)
}

/// IPC 内部实现：复用 `commands::timer` 的可测试实现。
fn timer_skip_ipc_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_skip_impl(state)
}

/// 托盘复用：开始计时的内部实现（不暴露给前端）。
pub fn timer_start_inner(state: &AppState) -> AppResult<()> {
    let _ = timer_start_ipc_impl(state)?;
    let _ = crate::tray::refresh_tray(state);
    Ok(())
}

/// 托盘复用：暂停计时的内部实现（不暴露给前端）。
pub fn timer_pause_inner(state: &AppState) -> AppResult<()> {
    let _ = timer_pause_ipc_impl(state)?;
    let _ = crate::tray::refresh_tray(state);
    Ok(())
}
