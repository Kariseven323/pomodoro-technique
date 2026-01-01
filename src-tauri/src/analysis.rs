//! 专注时段分析：基于历史记录统计时段/星期/标签效率，并生成摘要文案。

use std::collections::BTreeMap;

use chrono::{Datelike as _, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::app_data::{DateRange, HistoryDay};
use crate::errors::{AppError, AppResult};

/// 专注分析结果（用于前端图表渲染）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusAnalysis {
    /// 24 小时分布（按 `startTime` 的小时计数）。
    pub hourly_counts: Vec<u32>,
    /// 时段分布：`[0-6, 6-12, 12-18, 18-24]`。
    pub period_counts: Vec<u32>,
    /// 星期分布：`[周一..周日]`。
    pub weekday_counts: Vec<u32>,
    /// 交叉热力：`weekday_hour_counts[weekday][hour]`（7x24）。
    pub weekday_hour_counts: Vec<Vec<u32>>,
    /// 标签效率：各标签平均专注时长（分钟）。
    pub tag_efficiency: Vec<TagEfficiency>,
    /// 文字总结（示例：「你在上午 9-11 点专注效率最高」）。
    pub summary: String,
}

/// 标签效率条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEfficiency {
    /// 标签名。
    pub tag: String,
    /// 平均时长（分钟）。
    pub avg_duration: f64,
    /// 样本数（番茄数量）。
    pub count: u32,
}

/// 生成指定日期范围的专注分析（输入为按日分组的历史数据切片）。
pub fn get_focus_analysis(days: &[HistoryDay], range: &DateRange) -> AppResult<FocusAnalysis> {
    let (from, to) = parse_range(range)?;

    let mut hourly = vec![0u32; 24];
    let mut periods = vec![0u32; 4];
    let mut weekday_counts = vec![0u32; 7];
    let mut matrix = vec![vec![0u32; 24]; 7];

    let mut tag_total: BTreeMap<String, u32> = BTreeMap::new();
    let mut tag_count: BTreeMap<String, u32> = BTreeMap::new();

    for day in days {
        let day_date = match NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        if day_date < from || day_date > to {
            continue;
        }

        let weekday_index = weekday_to_index(day_date.weekday());
        for r in &day.records {
            let hour = parse_hour(&r.start_time).unwrap_or(0);
            hourly[hour] += 1;
            periods[period_index(hour)] += 1;
            weekday_counts[weekday_index] += 1;
            matrix[weekday_index][hour] += 1;

            *tag_total.entry(r.tag.clone()).or_insert(0) += r.duration;
            *tag_count.entry(r.tag.clone()).or_insert(0) += 1;
        }
    }

    let mut tag_efficiency: Vec<TagEfficiency> = tag_total
        .into_iter()
        .map(|(tag, total)| {
            let count = tag_count.get(&tag).copied().unwrap_or(0);
            let avg = if count == 0 {
                0.0
            } else {
                total as f64 / count as f64
            };
            TagEfficiency {
                tag,
                avg_duration: avg,
                count,
            }
        })
        .collect();

    // 让“标签效率”更直观：按样本数/平均时长排序。
    tag_efficiency.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.avg_duration.total_cmp(&a.avg_duration))
    });

    let summary = build_summary(&hourly);

    Ok(FocusAnalysis {
        hourly_counts: hourly,
        period_counts: periods,
        weekday_counts,
        weekday_hour_counts: matrix,
        tag_efficiency,
        summary,
    })
}

/// 解析日期范围，并确保 `from <= to`。
fn parse_range(range: &DateRange) -> AppResult<(NaiveDate, NaiveDate)> {
    let from = NaiveDate::parse_from_str(range.from.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("range.from 必须为 YYYY-MM-DD".to_string()))?;
    let to = NaiveDate::parse_from_str(range.to.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("range.to 必须为 YYYY-MM-DD".to_string()))?;
    if from > to {
        return Err(AppError::Validation(
            "日期范围不合法：from 不能晚于 to".to_string(),
        ));
    }
    Ok((from, to))
}

/// 从 `HH:mm` 中解析小时（0-23）。
fn parse_hour(hhmm: &str) -> Option<usize> {
    let hh = hhmm.split(':').next()?.trim();
    let hour: usize = hh.parse().ok()?;
    if hour < 24 {
        Some(hour)
    } else {
        None
    }
}

/// 将小时映射到 4 个时段桶：0-6 / 6-12 / 12-18 / 18-24。
fn period_index(hour: usize) -> usize {
    match hour {
        0..=5 => 0,
        6..=11 => 1,
        12..=17 => 2,
        _ => 3,
    }
}

/// 将 chrono `Weekday` 转为 `[周一..周日]` 索引。
fn weekday_to_index(weekday: Weekday) -> usize {
    match weekday {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

/// 生成摘要：取番茄数量最多的连续 2 小时窗口。
fn build_summary(hourly: &[u32]) -> String {
    if hourly.len() != 24 {
        return "暂无分析数据".to_string();
    }
    let total: u32 = hourly.iter().sum();
    if total == 0 {
        return "暂无分析数据".to_string();
    }

    let mut best_start = 0usize;
    let mut best_sum = 0u32;
    for start in 0..23 {
        let sum = hourly[start] + hourly[start + 1];
        if sum > best_sum {
            best_sum = sum;
            best_start = start;
        }
    }

    let label = time_range_label(best_start, best_start + 2);
    format!("你在{}专注效率最高", label)
}

/// 将小时范围格式化为更自然的中文时段描述（例如“上午 9-11 点”）。
fn time_range_label(from_hour: usize, to_hour: usize) -> String {
    let (prefix, display_from, display_to) = if from_hour < 6 {
        ("凌晨", from_hour, to_hour)
    } else if from_hour < 12 {
        ("上午", from_hour, to_hour)
    } else if from_hour < 18 {
        ("下午", from_hour - 12, to_hour - 12)
    } else {
        ("晚上", from_hour - 12, to_hour - 12)
    };

    // 处理 12/24 的显示：避免出现 “下午 0 点”。
    let normalize = |h: usize, is_pm: bool| -> usize {
        if is_pm && h == 0 {
            12
        } else {
            h
        }
    };
    let is_pm = prefix == "下午" || prefix == "晚上";
    let f = normalize(display_from, is_pm);
    let t = normalize(display_to, is_pm);
    format!("{prefix} {f}-{t} 点")
}
