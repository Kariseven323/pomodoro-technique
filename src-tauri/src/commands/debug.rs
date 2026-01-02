//! 调试相关命令：生成/清除测试历史数据（PRD v3）。

use crate::app_data::{HistoryDay, HistoryRecord, Phase, Settings};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;

/// 向前端广播“调试历史数据变更”的事件名（用于自动刷新历史页面）。
pub const EVENT_HISTORY_DEV_CHANGED: &str = "pomodoro://history_dev_changed";

/// 开发者命令：一键生成测试历史数据并写入 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_generate_history(state: tauri::State<'_, AppState>, days: u32) -> Result<u32, String> {
    to_ipc_result(debug_generate_history_impl(&state, days))
}

/// 开发者命令：清空 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_clear_history(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    to_ipc_result(debug_clear_history_impl(&state))
}

/// 生成调试历史数据的内部实现：校验参数、写入 store、并通知前端刷新。
fn debug_generate_history_impl(state: &AppState, days: u32) -> AppResult<u32> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation("仅开发环境可使用调试模式".to_string()));
    }
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

/// 清除调试历史数据的内部实现：清空 `history_dev` 并通知前端刷新。
fn debug_clear_history_impl(state: &AppState) -> AppResult<bool> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation("仅开发环境可使用调试模式".to_string()));
    }

    state.update_data(|data| {
        data.history_dev = Vec::new();
        Ok(())
    })?;

    tracing::info!(target: "storage", "已清除测试历史数据（history_dev）");
    let _ = state.emit_simple_event(EVENT_HISTORY_DEV_CHANGED);
    Ok(true)
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
