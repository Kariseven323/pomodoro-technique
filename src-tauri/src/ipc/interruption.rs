//! 中断相关 IPC 命令：记录中断、获取统计、读取 combo/累计番茄数（PRD v4）。

use crate::app_data::{DateRange, InterruptionDay, InterruptionRecord, InterruptionType};
use crate::commands::common::to_ipc_result;
use crate::errors::{AppError, AppResult};
use crate::interruptions::InterruptionStats;
use crate::state::AppState;

/// 记录一次中断（PRD v4：工作阶段中断）。
#[tauri::command]
pub fn record_interruption(
    state: tauri::State<'_, AppState>,
    reason: String,
    r#type: String,
) -> Result<InterruptionRecord, String> {
    to_ipc_result(record_interruption_impl(&state, reason, r#type))
}

/// 获取中断统计（用于“中断分析”卡片）。
#[tauri::command]
pub fn get_interruption_stats(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<InterruptionStats, String> {
    to_ipc_result(get_interruption_stats_impl(&state, &range))
}

/// 获取当前 Combo 数。
#[tauri::command]
pub fn get_combo(state: tauri::State<'_, AppState>) -> Result<u32, String> {
    to_ipc_result(Ok(state.data_snapshot().current_combo))
}

/// 获取累计完成番茄总数。
#[tauri::command]
pub fn get_total_pomodoros(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    to_ipc_result(Ok(state.data_snapshot().total_pomodoros))
}

/// `record_interruption` 的内部实现：生成记录并写入 `AppData.interruptions`。
fn record_interruption_impl(
    state: &AppState,
    reason: String,
    kind: String,
) -> AppResult<InterruptionRecord> {
    let kind = parse_interruption_type(&kind)?;
    let reason = reason.trim().to_string();

    let record = state.update_data_and_timer(
        |data, timer_runtime| {
            if !data.settings.interruption.enabled {
                return Err(AppError::Validation("已关闭“记录中断”".to_string()));
            }

            if timer_runtime.phase != crate::app_data::Phase::Work
                || !timer_runtime.is_work_started()
            {
                return Err(AppError::Validation(
                    "仅工作阶段进行中可记录中断".to_string(),
                ));
            }

            let remaining_seconds = timer_runtime.remaining_seconds;
            let focused_seconds = timer_runtime.focused_seconds(&data.settings);
            let tag = timer_runtime.current_tag.clone();

            let timestamp = chrono::Utc::now().to_rfc3339();
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();

            let record = InterruptionRecord {
                timestamp,
                remaining_seconds,
                focused_seconds,
                reason,
                r#type: kind,
                tag,
            };

            ensure_interruption_day(&mut data.interruptions, &date)
                .records
                .push(record.clone());

            // PRD v4：中断后 Combo 重置为 0。
            state.reset_combo_locked(data);

            Ok(record)
        },
        true,
    )?;

    Ok(record)
}

/// `get_interruption_stats` 的内部实现：基于快照计算。
fn get_interruption_stats_impl(
    state: &AppState,
    range: &DateRange,
) -> AppResult<InterruptionStats> {
    let data = state.data_snapshot();
    crate::interruptions::compute_interruption_stats(&data, range)
}

/// 将字符串解析为 `InterruptionType`（PRD v4：reset/skip/quit）。
fn parse_interruption_type(s: &str) -> AppResult<InterruptionType> {
    match s.trim() {
        "reset" => Ok(InterruptionType::Reset),
        "skip" => Ok(InterruptionType::Skip),
        "quit" => Ok(InterruptionType::Quit),
        _ => Err(AppError::Validation(
            "中断类型必须为 reset/skip/quit".to_string(),
        )),
    }
}

/// 在 `interruptions` 数组中确保存在指定日期的 `InterruptionDay`，并返回可变引用。
fn ensure_interruption_day<'a>(
    items: &'a mut Vec<InterruptionDay>,
    date: &str,
) -> &'a mut InterruptionDay {
    if let Some(index) = items.iter().position(|d| d.date == date) {
        return &mut items[index];
    }
    items.push(InterruptionDay {
        date: date.to_string(),
        records: Vec::new(),
    });
    let last = items.len().saturating_sub(1);
    &mut items[last]
}
