//! 设置相关命令：更新 settings、设置目标等。

use crate::app_data::{Phase, Settings};
use crate::errors::AppResult;
use crate::state::AppState;
use crate::timer;

use super::common::to_ipc_result;
use super::types::AppSnapshot;

/// 更新设置（带范围校验），并在必要时重置当前阶段的剩余时间。
#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<AppSnapshot, String> {
    to_ipc_result(update_settings_impl(&state, settings))
}

/// 更新设置的内部实现（便于统一错误处理与托盘复用）。
fn update_settings_impl(state: &AppState, settings: Settings) -> AppResult<AppSnapshot> {
    timer::validate_settings(&settings)?;

    tracing::info!(
        target: "storage",
        "更新设置：pomodoro={} shortBreak={} longBreak={} longBreakInterval={} dailyGoal={} weeklyGoal={} alwaysOnTop={}",
        settings.pomodoro,
        settings.short_break,
        settings.long_break,
        settings.long_break_interval,
        settings.daily_goal,
        settings.weekly_goal,
        settings.always_on_top
    );

    state.update_data_and_timer(
        |data, timer_runtime| {
            data.settings = settings.clone();

            // 若当前未运行，则根据阶段同步剩余时间，以保证 UI 与设置一致。
            if !timer_runtime.is_running {
                match timer_runtime.phase {
                    Phase::Work => {
                        timer_runtime.remaining_seconds = settings.pomodoro as u64 * 60;
                    }
                    Phase::ShortBreak => {
                        timer_runtime.remaining_seconds = settings.short_break as u64 * 60;
                    }
                    Phase::LongBreak => {
                        timer_runtime.remaining_seconds = settings.long_break as u64 * 60;
                    }
                }
            }
            Ok(())
        },
        true,
    )?;

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 设置每日/每周目标（0 表示不设目标），并持久化到 settings。
#[tauri::command]
pub fn set_goals(
    state: tauri::State<'_, AppState>,
    daily: u32,
    weekly: u32,
) -> Result<Settings, String> {
    to_ipc_result(set_goals_impl(&state, daily, weekly))
}

/// 设置目标的内部实现（便于统一错误处理）。
fn set_goals_impl(state: &AppState, daily: u32, weekly: u32) -> AppResult<Settings> {
    let mut next = state.data_snapshot().settings;
    next.daily_goal = daily;
    next.weekly_goal = weekly;
    timer::validate_settings(&next)?;

    state.update_data(|data| {
        data.settings.daily_goal = daily;
        data.settings.weekly_goal = weekly;
        Ok(())
    })?;

    tracing::info!(target: "timer", "设置目标：dailyGoal={} weeklyGoal={}", daily, weekly);
    let _ = state.emit_timer_snapshot();

    Ok(state.data_snapshot().settings)
}
