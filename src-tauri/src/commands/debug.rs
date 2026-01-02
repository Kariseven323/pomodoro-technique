//! 调试相关命令：生成/清除测试历史数据（PRD v3）。

use super::state_like::CommandState;
use crate::app_data::{HistoryDay, HistoryRecord, Phase, Settings};
use crate::errors::{AppError, AppResult};

/// 向前端广播“调试历史数据变更”的事件名（用于自动刷新历史页面）。
pub const EVENT_HISTORY_DEV_CHANGED: &str = "pomodoro://history_dev_changed";

/// 生成调试历史数据的内部实现：校验参数、写入 store、并通知前端刷新。
#[cfg(debug_assertions)]
pub(crate) fn debug_generate_history_impl<S: CommandState>(state: &S, days: u32) -> AppResult<u32> {
    if !(1..=365).contains(&days) {
        return Err(AppError::Validation("天数需在 1-365".to_string()));
    }

    let mut generated = 0u32;

    state.update_data(|data| {
        let settings = data.settings.clone();
        let tags = if data.tags.is_empty() {
            vec!["工作".to_string()]
        } else {
            data.tags.clone()
        };

        let (history, count) = generate_history_dev(days, &settings, &tags);
        data.history_dev = history;
        generated = count;
        Ok(())
    })?;

    tracing::info!(target: "storage", "生成测试历史数据：days={} records={}", days, generated);
    let _ = state.emit_simple_event(EVENT_HISTORY_DEV_CHANGED);
    Ok(generated)
}

/// 生成调试历史数据的内部实现：非开发环境直接拒绝（避免 release 包携带调试功能）。
#[cfg(not(debug_assertions))]
pub(crate) fn debug_generate_history_impl<S: CommandState>(
    _state: &S,
    _days: u32,
) -> AppResult<u32> {
    Err(AppError::Validation("仅开发环境可使用调试模式".to_string()))
}

/// 清除调试历史数据的内部实现：清空 `history_dev` 并通知前端刷新。
#[cfg(debug_assertions)]
pub(crate) fn debug_clear_history_impl<S: CommandState>(state: &S) -> AppResult<bool> {
    state.update_data(|data| {
        data.history_dev = Vec::new();
        Ok(())
    })?;

    tracing::info!(target: "storage", "已清除测试历史数据（history_dev）");
    let _ = state.emit_simple_event(EVENT_HISTORY_DEV_CHANGED);
    Ok(true)
}

/// 清除调试历史数据的内部实现：非开发环境直接拒绝（避免 release 包携带调试功能）。
#[cfg(not(debug_assertions))]
pub(crate) fn debug_clear_history_impl<S: CommandState>(_state: &S) -> AppResult<bool> {
    Err(AppError::Validation("仅开发环境可使用调试模式".to_string()))
}

/// 生成 `history_dev`：返回按日分组的历史与生成的记录总数。
fn generate_history_dev(days: u32, settings: &Settings, tags: &[String]) -> (Vec<HistoryDay>, u32) {
    use chrono::{Datelike as _, Duration as ChronoDuration, Local, NaiveDate, Weekday};
    use rand::Rng as _;

    let mut rng = rand::thread_rng();
    let today: NaiveDate = Local::now().date_naive();
    let start = today - ChronoDuration::days((days as i64).saturating_sub(1));

    let mut out: Vec<HistoryDay> = Vec::new();
    let mut total = 0u32;

    for offset in 0..days {
        let date = start + ChronoDuration::days(offset as i64);
        let weekday = date.weekday();
        let base = rng.gen_range(4..=8);
        let daily_count = if weekday == Weekday::Sat || weekday == Weekday::Sun {
            (base / 2).max(1)
        } else {
            base
        };

        let mut records: Vec<HistoryRecord> = Vec::new();
        for _ in 0..daily_count {
            let phase_roll: u8 = rng.gen_range(0..=99);
            let phase = if phase_roll < 80 {
                Phase::Work
            } else if phase_roll < 90 {
                Phase::ShortBreak
            } else {
                Phase::LongBreak
            };

            let duration = match phase {
                Phase::Work => {
                    let base = settings.pomodoro as i32;
                    let varied = base + rng.gen_range(-5..=5);
                    varied.clamp(1, 60) as u32
                }
                Phase::ShortBreak => settings.short_break.clamp(1, 30),
                Phase::LongBreak => settings.long_break.clamp(1, 60),
            };

            let (start_time, end_time) = random_time_in_windows(&mut rng, duration);
            let tag = pick_random_tag(&mut rng, tags);

            records.push(HistoryRecord {
                tag,
                start_time,
                end_time: Some(end_time),
                duration,
                phase,
                remark: String::new(),
            });
        }

        records.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        total += records.len() as u32;

        out.push(HistoryDay {
            date: date.format("%Y-%m-%d").to_string(),
            records,
        });
    }

    (out, total)
}

/// 从现有标签列表中随机挑选一个标签（列表为空时回退为“工作”）。
fn pick_random_tag(rng: &mut impl rand::Rng, tags: &[String]) -> String {
    if tags.is_empty() {
        return "工作".to_string();
    }
    let idx = rng.gen_range(0..tags.len());
    tags[idx].clone()
}

/// 在规定时间窗内随机生成开始/结束时间（HH:mm），并保证结束时间不超过窗末尾。
fn random_time_in_windows(rng: &mut impl rand::Rng, duration_minutes: u32) -> (String, String) {
    // 规则：9:00-12:00, 14:00-18:00
    let windows: &[(u32, u32)] = &[(9 * 60, 12 * 60), (14 * 60, 18 * 60)];
    let (start_min, end_min) = windows[rng.gen_range(0..windows.len())];
    let latest_start = end_min.saturating_sub(duration_minutes).max(start_min);
    let start = rng.gen_range(start_min..=latest_start);
    let end = start + duration_minutes;
    (minutes_to_hhmm(start), minutes_to_hhmm(end))
}

/// 将分钟数（0-1440）格式化为 `HH:mm`。
fn minutes_to_hhmm(total_minutes: u32) -> String {
    let hh = (total_minutes / 60) % 24;
    let mm = total_minutes % 60;
    format!("{:02}:{:02}", hh, mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng as _;

    use crate::app_data::AppData;
    use crate::commands::state_like::TestState;

    /// `minutes_to_hhmm`：应正确补零并按 24 小时制取模。
    #[test]
    fn minutes_to_hhmm_formats_correctly() {
        assert_eq!(minutes_to_hhmm(0), "00:00");
        assert_eq!(minutes_to_hhmm(9 * 60 + 5), "09:05");
        assert_eq!(minutes_to_hhmm(23 * 60 + 59), "23:59");
        assert_eq!(minutes_to_hhmm(24 * 60), "00:00");
    }

    /// `pick_random_tag`：空列表时应回退为“工作”。
    #[test]
    fn pick_random_tag_falls_back_when_empty() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let out = pick_random_tag(&mut rng, &[]);
        assert_eq!(out, "工作");
    }

    /// `generate_history_dev`：应生成指定天数的数据并返回记录总数。
    #[test]
    fn generate_history_dev_generates_days_and_counts() {
        let settings = Settings::default();
        let tags = vec!["A".to_string(), "B".to_string()];
        let (days, total) = generate_history_dev(5, &settings, &tags);
        assert_eq!(days.len() as u32, 5);
        assert!(total > 0);
        assert!(days.iter().all(|d| !d.records.is_empty()));
    }

    /// `debug_generate_history_impl`：应写入 `history_dev` 并触发刷新事件。
    #[test]
    fn debug_generate_history_writes_and_emits() {
        let state = TestState::new(AppData::default());
        let count = debug_generate_history_impl(&state, 7).unwrap();
        assert!(count > 0);
        assert!(!state.data_snapshot().history_dev.is_empty());
        assert!(state
            .take_events()
            .iter()
            .any(|e| e == EVENT_HISTORY_DEV_CHANGED));
    }

    /// `debug_clear_history_impl`：应清空 `history_dev` 并触发刷新事件。
    #[test]
    fn debug_clear_history_clears_and_emits() {
        let mut data = AppData::default();
        data.history_dev = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: Vec::new(),
        }];
        let state = TestState::new(data);

        let ok = debug_clear_history_impl(&state).unwrap();
        assert!(ok);
        assert!(state.data_snapshot().history_dev.is_empty());
        assert!(state
            .take_events()
            .iter()
            .any(|e| e == EVENT_HISTORY_DEV_CHANGED));
    }

    /// `debug_generate_history_impl`：非法天数范围应返回校验错误。
    #[test]
    fn debug_generate_history_rejects_invalid_days() {
        let state = TestState::new(AppData::default());
        let err = debug_generate_history_impl(&state, 0).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }
}
