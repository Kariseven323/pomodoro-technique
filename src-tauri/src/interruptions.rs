//! 中断记录统计：频率、时段、原因分布与中断率等（PRD v4）。

use std::collections::BTreeMap;

use chrono::{Datelike as _, NaiveDate, Timelike as _};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app_data::{AppData, DateRange, InterruptionDay, InterruptionRecord};
use crate::commands::validation::{history_for_ui, validate_date_range};
use crate::errors::{AppError, AppResult};

/// 原因统计条目（用于饼图/列表展示）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct InterruptionReasonCount {
    /// 原因名称（空值会被规范化为 `未填写`）。
    pub reason: String,
    /// 次数。
    pub count: u32,
}

/// 中断统计（PRD v4：用于“中断分析”卡片）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct InterruptionStats {
    /// 中断总次数。
    pub total_interruptions: u32,
    /// 每日平均中断次数（按日期范围天数归一）。
    pub daily_average: f64,
    /// 每周平均中断次数（按范围覆盖到的周数归一）。
    pub weekly_average: f64,
    /// 24 小时分布（0-23 点）。
    pub hourly_counts: Vec<u32>,
    /// 原因分布（按次数倒序）。
    pub reason_distribution: Vec<InterruptionReasonCount>,
    /// 中断率：中断番茄数 / 总开始番茄数（开始=完成+中断）。
    pub interruption_rate: f64,
    /// 平均专注时长（秒；仅统计中断记录）。
    pub average_focused_seconds: f64,
}

/// 计算指定日期范围内的中断统计（闭区间）。
pub fn compute_interruption_stats(
    data: &AppData,
    range: &DateRange,
) -> AppResult<InterruptionStats> {
    validate_date_range(range)?;

    let records = collect_records_in_range(&data.interruptions, range);
    let total_interruptions = records.len() as u32;

    let day_count = day_count_inclusive(&range.from, &range.to)?;
    let week_count = week_count_covered(&range.from, &range.to)?;

    let mut hourly_counts = vec![0u32; 24];
    let mut reason_map = BTreeMap::<String, u32>::new();
    let mut focused_sum: u64 = 0;

    for r in &records {
        if let Some(hour) = hour_from_timestamp(&r.timestamp) {
            if let Some(slot) = hourly_counts.get_mut(hour as usize) {
                *slot = slot.saturating_add(1);
            }
        }
        let key = normalize_reason(&r.reason);
        reason_map
            .entry(key)
            .and_modify(|v| *v = v.saturating_add(1))
            .or_insert(1);
        focused_sum = focused_sum.saturating_add(r.focused_seconds);
    }

    let mut reason_distribution: Vec<InterruptionReasonCount> = reason_map
        .into_iter()
        .map(|(reason, count)| InterruptionReasonCount { reason, count })
        .collect();
    reason_distribution.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));

    let completed = completed_pomodoros_in_range(data, range);
    let started = completed.saturating_add(total_interruptions);
    let interruption_rate = if started == 0 {
        0.0
    } else {
        total_interruptions as f64 / started as f64
    };

    let average_focused_seconds = if total_interruptions == 0 {
        0.0
    } else {
        focused_sum as f64 / total_interruptions as f64
    };

    Ok(InterruptionStats {
        total_interruptions,
        daily_average: total_interruptions as f64 / day_count as f64,
        weekly_average: total_interruptions as f64 / week_count as f64,
        hourly_counts,
        reason_distribution,
        interruption_rate,
        average_focused_seconds,
    })
}

/// 在给定范围内收集中断记录（按 day 过滤后扁平化）。
fn collect_records_in_range(
    days: &[InterruptionDay],
    range: &DateRange,
) -> Vec<InterruptionRecord> {
    let mut out = Vec::new();
    for d in days
        .iter()
        .filter(|d| d.date >= range.from && d.date <= range.to)
    {
        out.extend(d.records.clone());
    }
    out
}

/// 计算范围天数（闭区间）。
fn day_count_inclusive(from: &str, to: &str) -> AppResult<u32> {
    let f = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;
    let t = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;
    let diff = (t - f).num_days();
    Ok((diff.max(0) as u32).saturating_add(1))
}

/// 计算范围覆盖的周数（按 ISO week 统计，包含起止所在周）。
fn week_count_covered(from: &str, to: &str) -> AppResult<u32> {
    let f = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;
    let t = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;

    let mut cur = f;
    let mut seen = std::collections::BTreeSet::<(i32, u32)>::new();
    while cur <= t {
        let w = cur.iso_week();
        seen.insert((w.year(), w.week()));
        if let Some(next) = cur.succ_opt() {
            cur = next;
        } else {
            break;
        }
    }
    Ok(seen.len().max(1) as u32)
}

/// 从 ISO 8601 时间戳解析小时（失败则返回 `None`）。
fn hour_from_timestamp(ts: &str) -> Option<u32> {
    let dt = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    Some(dt.with_timezone(&chrono::Local).hour())
}

/// 规范化原因字段：trim，空串替换为 `未填写`。
fn normalize_reason(reason: &str) -> String {
    let r = reason.trim();
    if r.is_empty() {
        "未填写".to_string()
    } else {
        r.to_string()
    }
}

/// 统计范围内完成番茄数（与历史页面一致：开发环境优先 `history_dev`）。
fn completed_pomodoros_in_range(data: &AppData, range: &DateRange) -> u32 {
    history_for_ui(data)
        .iter()
        .filter(|d| d.date >= range.from && d.date <= range.to)
        .map(|d| d.records.len() as u32)
        .sum()
}
