//! 专注时段分析：基于历史记录统计时段/星期/标签效率，并生成摘要文案。

use std::collections::BTreeMap;

use chrono::{Datelike as _, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app_data::{DateRange, HistoryDay};
use crate::errors::{AppError, AppResult};

/// 专注分析结果（用于前端图表渲染）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
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
            let count = tag_count
                .get(&tag)
                .copied()
                .expect("tag_total 与 tag_count 应保持键一致");
            let avg = total as f64 / count as f64;
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::{HistoryRecord, Phase};

    /// 构造一条最小的历史记录（用于专注分析测试）。
    fn record(tag: &str, start_time: &str, duration: u32) -> HistoryRecord {
        HistoryRecord {
            tag: tag.to_string(),
            start_time: start_time.to_string(),
            end_time: None,
            duration,
            phase: Phase::Work,
            remark: String::new(),
        }
    }

    /// `parse_range`：合法范围应通过，并返回正确的 NaiveDate。
    #[test]
    fn parse_range_accepts_valid_range() {
        let (from, to) = parse_range(&DateRange {
            from: "2025-01-01".to_string(),
            to: "2025-01-07".to_string(),
        })
        .unwrap();
        assert_eq!(from, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(to, NaiveDate::from_ymd_opt(2025, 1, 7).unwrap());
    }

    /// `parse_range`：当 from > to 时应返回校验错误。
    #[test]
    fn parse_range_rejects_from_after_to() {
        let err = parse_range(&DateRange {
            from: "2025-01-08".to_string(),
            to: "2025-01-07".to_string(),
        })
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `parse_range`：日期格式错误应返回校验错误。
    #[test]
    fn parse_range_rejects_invalid_format() {
        let err = parse_range(&DateRange {
            from: "2025/01/01".to_string(),
            to: "2025-01-07".to_string(),
        })
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `parse_hour`：应支持边界值（00:00/23:59）与常见格式。
    #[test]
    fn parse_hour_supports_boundaries() {
        assert_eq!(parse_hour("00:00"), Some(0));
        assert_eq!(parse_hour("23:59"), Some(23));
        assert_eq!(parse_hour(" 9:01 "), Some(9));
    }

    /// `parse_hour`：非法格式应返回 None。
    #[test]
    fn parse_hour_rejects_invalid_format() {
        assert_eq!(parse_hour(""), None);
        assert_eq!(parse_hour("xx:yy"), None);
        assert_eq!(parse_hour("24:00"), None);
        assert_eq!(parse_hour("12"), Some(12));
    }

    /// `period_index`：应按 0/6/12/18 边界映射到 4 个时段桶。
    #[test]
    fn period_index_respects_boundaries() {
        assert_eq!(period_index(0), 0);
        assert_eq!(period_index(5), 0);
        assert_eq!(period_index(6), 1);
        assert_eq!(period_index(11), 1);
        assert_eq!(period_index(12), 2);
        assert_eq!(period_index(17), 2);
        assert_eq!(period_index(18), 3);
        assert_eq!(period_index(23), 3);
    }

    /// `weekday_to_index`：应将周一到周日映射为 0..=6。
    #[test]
    fn weekday_to_index_maps_mon_to_sun() {
        assert_eq!(weekday_to_index(Weekday::Mon), 0);
        assert_eq!(weekday_to_index(Weekday::Tue), 1);
        assert_eq!(weekday_to_index(Weekday::Wed), 2);
        assert_eq!(weekday_to_index(Weekday::Thu), 3);
        assert_eq!(weekday_to_index(Weekday::Fri), 4);
        assert_eq!(weekday_to_index(Weekday::Sat), 5);
        assert_eq!(weekday_to_index(Weekday::Sun), 6);
    }

    /// `build_summary`：空数据应返回“暂无分析数据”。
    #[test]
    fn build_summary_returns_no_data_when_empty() {
        assert_eq!(build_summary(&vec![0u32; 24]), "暂无分析数据");
    }

    /// `build_summary`：当输入长度不是 24 时应返回“暂无分析数据”。
    #[test]
    fn build_summary_returns_no_data_when_len_is_not_24() {
        assert_eq!(build_summary(&vec![0u32; 0]), "暂无分析数据");
        assert_eq!(build_summary(&vec![0u32; 23]), "暂无分析数据");
        assert_eq!(build_summary(&vec![0u32; 25]), "暂无分析数据");
    }

    /// `build_summary`：应选择番茄数量最多的连续 2 小时窗口。
    #[test]
    fn build_summary_picks_best_two_hour_window() {
        let mut hourly = vec![0u32; 24];
        hourly[8] = 1;
        hourly[9] = 3;
        hourly[10] = 3; // 9-11 窗口合计 6，为最大
        hourly[11] = 1;
        let summary = build_summary(&hourly);
        assert!(summary.contains("专注效率最高"));
        assert!(summary.contains("上午 9-11 点"));
    }

    /// `time_range_label`：应按凌晨/上午/下午/晚上输出，并处理 12 点边界。
    #[test]
    fn time_range_label_formats_periods_and_12_boundary() {
        assert_eq!(time_range_label(0, 2), "凌晨 0-2 点");
        assert_eq!(time_range_label(6, 8), "上午 6-8 点");
        assert_eq!(time_range_label(12, 14), "下午 12-2 点");
        assert_eq!(time_range_label(18, 20), "晚上 6-8 点");
    }

    /// `get_focus_analysis`：空历史应返回全 0 分布与“暂无分析数据”摘要。
    #[test]
    fn get_focus_analysis_handles_empty_days() {
        let out = get_focus_analysis(
            &[],
            &DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-07".to_string(),
            },
        )
        .unwrap();
        assert_eq!(out.hourly_counts, vec![0u32; 24]);
        assert_eq!(out.period_counts, vec![0u32; 4]);
        assert_eq!(out.weekday_counts, vec![0u32; 7]);
        assert_eq!(out.weekday_hour_counts.len(), 7);
        assert_eq!(out.weekday_hour_counts[0].len(), 24);
        assert_eq!(out.tag_efficiency.len(), 0);
        assert_eq!(out.summary, "暂无分析数据");
    }

    /// `get_focus_analysis`：单日数据应正确累计小时/时段/星期与标签效率。
    #[test]
    fn get_focus_analysis_handles_single_day() {
        let days = vec![HistoryDay {
            date: "2025-01-01".to_string(), // 2025-01-01 是周三
            records: vec![
                record("学习", "09:10", 25),
                record("学习", "09:40", 30),
                record("工作", "10:00", 15),
            ],
        }];

        let out = get_focus_analysis(
            &days,
            &DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-01".to_string(),
            },
        )
        .unwrap();

        assert_eq!(out.hourly_counts[9], 2);
        assert_eq!(out.hourly_counts[10], 1);
        assert_eq!(out.period_counts[1], 3); // 6-12

        // 周三 -> index 2
        assert_eq!(out.weekday_counts[2], 3);
        assert_eq!(out.weekday_hour_counts[2][9], 2);
        assert_eq!(out.weekday_hour_counts[2][10], 1);

        // 标签效率按样本数排序：学习(2) 在前
        assert_eq!(out.tag_efficiency[0].tag, "学习");
        assert_eq!(out.tag_efficiency[0].count, 2);
        assert!((out.tag_efficiency[0].avg_duration - 27.5).abs() < 1e-9);
        assert_eq!(out.tag_efficiency[1].tag, "工作");
        assert_eq!(out.tag_efficiency[1].count, 1);
        assert!((out.tag_efficiency[1].avg_duration - 15.0).abs() < 1e-9);
    }

    /// `get_focus_analysis`：日期范围应为闭区间，并忽略范围外数据与非法日期。
    #[test]
    fn get_focus_analysis_respects_inclusive_range_and_skips_invalid_dates() {
        let days = vec![
            HistoryDay {
                date: "2025-01-01".to_string(),
                records: vec![record("A", "08:00", 25)],
            },
            HistoryDay {
                date: "2025-01-02".to_string(),
                records: vec![record("B", "09:00", 25)],
            },
            HistoryDay {
                date: "2025-01-03".to_string(),
                records: vec![record("C", "10:00", 25)],
            },
            HistoryDay {
                date: "bad-date".to_string(),
                records: vec![record("D", "11:00", 25)],
            },
        ];

        let out = get_focus_analysis(
            &days,
            &DateRange {
                from: "2025-01-02".to_string(),
                to: "2025-01-03".to_string(),
            },
        )
        .unwrap();

        assert_eq!(out.hourly_counts[8], 0);
        assert_eq!(out.hourly_counts[9], 1);
        assert_eq!(out.hourly_counts[10], 1);
        assert_eq!(out.tag_efficiency.len(), 2);
        assert_eq!(out.tag_efficiency[0].tag, "B");
        assert_eq!(out.tag_efficiency[1].tag, "C");
    }
}
