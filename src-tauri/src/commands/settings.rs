//! 设置相关命令：更新 settings、设置目标等。

use crate::app_data::{Phase, Settings};
use crate::errors::AppResult;
use crate::timer;

use super::state_like::CommandState;
use super::types::AppSnapshot;

/// 更新设置的内部实现（便于统一错误处理与托盘复用）。
pub(crate) fn update_settings_impl<S: CommandState>(
    state: &S,
    settings: Settings,
) -> AppResult<AppSnapshot> {
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

    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 设置目标的内部实现（便于统一错误处理）。
pub(crate) fn set_goals_impl<S: CommandState>(
    state: &S,
    daily: u32,
    weekly: u32,
) -> AppResult<Settings> {
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Once;

    use crate::app_data::AppData;
    use crate::commands::state_like::CommandState;
    use crate::commands::state_like::TestState;

    /// 初始化 `tracing`（仅一次）：确保 `tracing::info!` 的字段参数会被求值，便于覆盖率统计。
    fn init_tracing_once() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .with_test_writer()
                .try_init();
        });
    }

    /// `update_settings_impl`：应校验设置参数，并在未运行时同步当前阶段剩余时间。
    #[test]
    fn update_settings_updates_data_and_remaining_when_not_running() {
        init_tracing_once();
        let state = TestState::new(AppData::default());

        // 默认 phase=Work 且未运行，更新 pomodoro 后应同步 remaining_seconds。
        let mut next = state.data_snapshot().settings;
        next.pomodoro = 10;
        let snapshot = update_settings_impl(&state, next.clone()).unwrap();
        assert_eq!(snapshot.data.settings.pomodoro, 10);
        assert_eq!(snapshot.timer.remaining_seconds, 10 * 60);

        // 切到短休息阶段且未运行，更新 short_break 后应同步 remaining_seconds。
        state
            .update_timer(|t, _d| {
                t.phase = Phase::ShortBreak;
                t.is_running = false;
                Ok(())
            })
            .unwrap();
        let mut next2 = state.data_snapshot().settings;
        next2.short_break = 7;
        let snapshot2 = update_settings_impl(&state, next2.clone()).unwrap();
        assert_eq!(snapshot2.timer.phase, Phase::ShortBreak);
        assert_eq!(snapshot2.timer.remaining_seconds, 7 * 60);

        // 切到长休息阶段且未运行，更新 long_break 后应同步 remaining_seconds。
        state
            .update_timer(|t, _d| {
                t.phase = Phase::LongBreak;
                t.is_running = false;
                Ok(())
            })
            .unwrap();
        let mut next3 = state.data_snapshot().settings;
        next3.long_break = 9;
        let snapshot3 = update_settings_impl(&state, next3.clone()).unwrap();
        assert_eq!(snapshot3.timer.phase, Phase::LongBreak);
        assert_eq!(snapshot3.timer.remaining_seconds, 9 * 60);
    }

    /// `update_settings_impl`：运行中不应强制改写 remaining_seconds（避免打断当前倒计时）。
    #[test]
    fn update_settings_does_not_override_remaining_when_running() {
        let state = TestState::new(AppData::default());
        state
            .update_timer(|t, _d| {
                t.phase = Phase::Work;
                t.is_running = true;
                t.remaining_seconds = 3;
                Ok(())
            })
            .unwrap();

        let mut next = state.data_snapshot().settings;
        next.pomodoro = 50;
        let snapshot = update_settings_impl(&state, next).unwrap();
        assert_eq!(snapshot.timer.remaining_seconds, 3);
    }

    /// `update_settings_impl`：非法设置应返回校验错误。
    #[test]
    fn update_settings_rejects_invalid_settings() {
        let state = TestState::new(AppData::default());
        let mut next = state.data_snapshot().settings;
        next.pomodoro = 0;
        let err = update_settings_impl(&state, next).unwrap_err();
        assert!(matches!(err, crate::errors::AppError::Validation(_)));
    }

    /// `set_goals_impl`：应写入 daily/weekly 目标并返回最新 settings。
    #[test]
    fn set_goals_updates_settings() {
        let state = TestState::new(AppData::default());
        let out = set_goals_impl(&state, 3, 7).unwrap();
        assert_eq!(out.daily_goal, 3);
        assert_eq!(out.weekly_goal, 7);
        assert_eq!(state.data_snapshot().settings.daily_goal, 3);
        assert_eq!(state.data_snapshot().settings.weekly_goal, 7);
    }

    /// `set_goals_impl`：非法目标值（超出后端约束）应返回校验错误。
    #[test]
    fn set_goals_rejects_invalid_goal_ranges() {
        let state = TestState::new(AppData::default());
        let err = set_goals_impl(&state, 1001, 0).unwrap_err();
        assert!(matches!(err, crate::errors::AppError::Validation(_)));
    }
}
