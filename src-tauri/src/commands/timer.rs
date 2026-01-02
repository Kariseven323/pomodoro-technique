//! 计时器相关命令：开始/暂停/重置/跳过（供前端与托盘复用）。

use crate::app_data::Phase;
use crate::errors::AppResult;
use crate::timer::{compute_today_stats, TimerClock, TimerSnapshot};

use super::state_like::CommandState;

/// 开始计时的可测试实现：不依赖托盘；内部会广播快照事件。
pub(crate) fn timer_start_impl<S: CommandState>(state: &S) -> AppResult<TimerSnapshot> {
    timer_start_transition_with_deps(state, &crate::timer::SystemClock, |names| {
        crate::processes::kill_names_best_effort(names)
    })?;
    Ok(state.timer_snapshot())
}

/// 开始计时的可测试实现：可注入 clock 与 kill 函数（避免测试中触发系统调用）。
fn timer_start_transition_with_deps<S: CommandState>(
    state: &S,
    clock: &dyn TimerClock,
    kill_names: impl FnOnce(&[String]) -> crate::processes::KillSummary,
) -> AppResult<()> {
    let (names_to_kill, should_kill) = state.update_data_and_timer(
        |data, timer_runtime| {
            let should_kill = timer_runtime.phase == Phase::Work
                && !timer_runtime.blacklist_locked()
                && !timer_runtime.is_running;
            let names: Vec<String> = data.blacklist.iter().map(|b| b.name.clone()).collect();
            timer_runtime.start(&data.settings, clock);
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
        let payload = kill_names(&names_to_kill);
        let _ = state.emit_kill_result(payload);
    }

    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 暂停计时的可测试实现：不依赖托盘；内部会广播快照事件。
pub(crate) fn timer_pause_impl<S: CommandState>(state: &S) -> AppResult<TimerSnapshot> {
    timer_pause_transition(state)?;
    Ok(state.timer_snapshot())
}

/// 暂停计时的可测试实现：不依赖托盘与系统资源。
fn timer_pause_transition<S: CommandState>(state: &S) -> AppResult<()> {
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

    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 重置计时的内部实现（便于统一错误处理）。
pub(crate) fn timer_reset_impl<S: CommandState>(state: &S) -> AppResult<TimerSnapshot> {
    timer_reset_transition(state)?;
    Ok(state.timer_snapshot())
}

/// 重置计时的可测试实现：不依赖托盘与系统资源。
fn timer_reset_transition<S: CommandState>(state: &S) -> AppResult<()> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.reset(&data.settings);
            Ok(())
        },
        false,
    )?;

    tracing::info!(target: "timer", "重置计时器：回到工作阶段");
    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 跳过阶段的内部实现（便于统一错误处理）。
pub(crate) fn timer_skip_impl<S: CommandState>(state: &S) -> AppResult<TimerSnapshot> {
    timer_skip_transition_with_clock(state, &crate::timer::SystemClock)?;
    Ok(state.timer_snapshot())
}

/// 跳过阶段的可测试实现：可注入 `clock`，避免依赖真实日期。
fn timer_skip_transition_with_clock<S: CommandState>(
    state: &S,
    clock: &dyn TimerClock,
) -> AppResult<()> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            let today = clock.today_date();
            let completed_today = compute_today_stats(data, &today).total;
            timer_runtime.skip(&data.settings, completed_today);
            Ok(())
        },
        false,
    )?;

    tracing::info!(target: "timer", "跳过阶段：phase={:?}", state.timer_snapshot().phase);
    let _ = state.emit_timer_snapshot();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Once;

    use crate::app_data::{AppData, BlacklistItem, HistoryDay, HistoryRecord, Phase, Settings};
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

    /// 固定时钟：用于让命令层测试稳定可复现。
    struct FixedClock {
        today: String,
        now: String,
        week_from: String,
        week_to: String,
    }

    impl FixedClock {
        /// 构造一个固定时钟（weekRange 默认覆盖 today）。
        fn new(today: &str, now: &str) -> Self {
            Self {
                today: today.to_string(),
                now: now.to_string(),
                week_from: today.to_string(),
                week_to: today.to_string(),
            }
        }
    }

    impl TimerClock for FixedClock {
        /// 返回固定日期。
        fn today_date(&self) -> String {
            self.today.clone()
        }

        /// 返回固定时间。
        fn now_hhmm(&self) -> String {
            self.now.clone()
        }

        /// 返回固定周范围。
        fn current_week_range(&self) -> (String, String) {
            (self.week_from.clone(), self.week_to.clone())
        }
    }

    /// `timer_start_transition_with_deps`：工作阶段首次开始应触发 kill，并广播快照。
    #[test]
    fn timer_start_kills_on_first_work_start() {
        init_tracing_once();
        let mut data = AppData::default();
        data.blacklist = vec![BlacklistItem {
            name: "a.exe".to_string(),
            display_name: "A".to_string(),
        }];
        data.settings = Settings {
            pomodoro: 1,
            ..data.settings
        };
        let state = TestState::new(data);
        let clock = FixedClock::new("2025-01-01", "09:00");

        timer_start_transition_with_deps(&state, &clock, |names| crate::processes::KillSummary {
            items: vec![crate::processes::termination::KillItem {
                name: names[0].clone(),
                pids: vec![1],
                killed: 1,
                failed: 0,
                requires_admin: false,
            }],
            requires_admin: false,
        })
        .unwrap();

        assert!(state.timer_snapshot().is_running);
        assert_eq!(state.emitted_timer_snapshot_count(), 1);
        let kills = state.take_kill_results();
        assert_eq!(kills.len(), 1);
        assert_eq!(kills[0].items[0].name, "a.exe");

        // 再次开始：不应重复 kill。
        timer_start_transition_with_deps(&state, &clock, |_names| unreachable!("不应再触发 kill"))
            .unwrap();
        assert_eq!(state.take_kill_results().len(), 0);
        assert_eq!(state.emitted_timer_snapshot_count(), 2);
    }

    /// `timer_start_impl`：黑名单为空时也应能启动并返回快照（不会触发系统进程终止）。
    #[test]
    fn timer_start_impl_starts_and_returns_snapshot() {
        let state = TestState::new(AppData::default());
        let snapshot = timer_start_impl(&state).unwrap();
        assert!(snapshot.is_running);
        assert_eq!(state.emitted_timer_snapshot_count(), 1);
    }

    /// `timer_pause_transition`：暂停应设置 is_running=false 并广播快照。
    #[test]
    fn timer_pause_pauses_and_emits() {
        let state = TestState::new(AppData::default());
        state
            .update_timer(|t, _d| {
                t.is_running = true;
                Ok(())
            })
            .unwrap();

        let snapshot = timer_pause_impl(&state).unwrap();
        assert!(!snapshot.is_running);
        assert!(!state.timer_snapshot().is_running);
        assert_eq!(state.emitted_timer_snapshot_count(), 1);
    }

    /// `timer_reset_transition`：应回到工作阶段初始剩余时间并广播快照。
    #[test]
    fn timer_reset_resets_and_emits() {
        let mut data = AppData::default();
        data.settings.pomodoro = 2;
        let state = TestState::new(data);
        state
            .update_timer(|t, _d| {
                t.phase = Phase::LongBreak;
                t.remaining_seconds = 1;
                t.is_running = true;
                Ok(())
            })
            .unwrap();

        let snapshot = timer_reset_impl(&state).unwrap();
        assert_eq!(snapshot.phase, Phase::Work);
        let snap = state.timer_snapshot();
        assert_eq!(snap.phase, Phase::Work);
        assert_eq!(snap.remaining_seconds, 2 * 60);
        assert!(!snap.is_running);
        assert_eq!(state.emitted_timer_snapshot_count(), 1);
    }

    /// `timer_skip_transition_with_clock`：应按 completed_today 与 long_break_interval 切换阶段并广播快照。
    #[test]
    fn timer_skip_switches_phase_and_emits() {
        let mut data = AppData::default();
        data.settings.short_break = 1;
        data.settings.long_break = 2;
        data.settings.long_break_interval = 2;
        data.history = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "学习".to_string(),
                start_time: "08:00".to_string(),
                end_time: Some("08:25".to_string()),
                duration: 25,
                phase: Phase::Work,
                remark: String::new(),
            }],
        }];
        let state = TestState::new(data);
        let clock = FixedClock::new("2025-01-01", "09:00");

        // completed_today=1，跳过工作阶段后 completed_today_after=1，不是 2 的倍数 -> 短休息。
        timer_skip_transition_with_clock(&state, &clock).unwrap();
        assert_eq!(state.timer_snapshot().phase, Phase::ShortBreak);
        assert_eq!(state.emitted_timer_snapshot_count(), 1);
    }
}
