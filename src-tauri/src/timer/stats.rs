//! 计时器统计计算：今日/本周数据与目标进度。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app_data::{AppData, Phase};

/// 标签计数条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TagCount {
    /// 标签名。
    pub tag: String,
    /// 完成次数。
    pub count: u32,
}

/// 今日统计（总数 + 按标签分组）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TodayStats {
    /// 今日完成的番茄总数。
    pub total: u32,
    /// 按标签统计。
    pub by_tag: Vec<TagCount>,
}

/// 本周统计（总数 + 按标签分组）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WeekStats {
    /// 本周完成的番茄总数。
    pub total: u32,
    /// 按标签统计。
    pub by_tag: Vec<TagCount>,
}

/// 目标进度（每日/每周）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GoalProgress {
    /// 每日目标（0 表示未设置）。
    pub daily_goal: u32,
    /// 今日已完成。
    pub daily_completed: u32,
    /// 每周目标（0 表示未设置）。
    pub weekly_goal: u32,
    /// 本周已完成。
    pub weekly_completed: u32,
}

/// 计算指定日期（YYYY-MM-DD）的“今日统计”（仅统计工作阶段记录）。
pub fn compute_today_stats(data: &AppData, today: &str) -> TodayStats {
    let mut map: BTreeMap<String, u32> = BTreeMap::new();
    let mut total = 0u32;

    if let Some(day) = data.history.iter().find(|d| d.date == today) {
        for r in &day.records {
            if r.phase != Phase::Work {
                continue;
            }
            total += 1;
            *map.entry(r.tag.clone()).or_insert(0) += 1;
        }
    }

    TodayStats {
        total,
        by_tag: map
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect(),
    }
}

/// 计算闭区间 `[from, to]`（YYYY-MM-DD）的“本周统计”（仅统计工作阶段记录）。
pub fn compute_week_stats(data: &AppData, from: &str, to: &str) -> WeekStats {
    let mut map: BTreeMap<String, u32> = BTreeMap::new();
    let mut total = 0u32;

    for day in &data.history {
        if day.date.as_str() < from || day.date.as_str() > to {
            continue;
        }
        for r in &day.records {
            if r.phase != Phase::Work {
                continue;
            }
            total += 1;
            *map.entry(r.tag.clone()).or_insert(0) += 1;
        }
    }

    WeekStats {
        total,
        by_tag: map
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_data::{HistoryDay, HistoryRecord};

    /// 构造一条测试用历史记录（默认仅填充必要字段）。
    fn record(tag: &str, phase: Phase) -> HistoryRecord {
        HistoryRecord {
            tag: tag.to_string(),
            start_time: "09:00".to_string(),
            end_time: Some("09:25".to_string()),
            duration: 25,
            phase,
            remark: String::new(),
        }
    }

    /// `compute_today_stats` 只统计工作阶段，并按标签汇总。
    #[test]
    fn compute_today_stats_counts_only_work() {
        let data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: vec![
                    record("学习", Phase::Work),
                    record("工作", Phase::Work),
                    record("学习", Phase::Work),
                    record("短休息", Phase::ShortBreak),
                ],
            }],
            ..AppData::default()
        };

        let out = compute_today_stats(&data, "2025-01-01");
        assert_eq!(out.total, 3);
        assert_eq!(
            out.by_tag,
            vec![
                TagCount {
                    tag: "学习".to_string(),
                    count: 2
                },
                TagCount {
                    tag: "工作".to_string(),
                    count: 1
                }
            ]
        );
    }

    /// `compute_week_stats` 按日期闭区间聚合，并忽略区间外的天。
    #[test]
    fn compute_week_stats_respects_range() {
        let data = AppData {
            history: vec![
                HistoryDay {
                    date: "2025-01-01".to_string(),
                    records: vec![record("A", Phase::Work)],
                },
                HistoryDay {
                    date: "2025-01-03".to_string(),
                    records: vec![record("A", Phase::Work), record("B", Phase::Work)],
                },
                HistoryDay {
                    date: "2025-01-10".to_string(),
                    records: vec![record("A", Phase::Work)],
                },
            ],
            ..AppData::default()
        };

        let out = compute_week_stats(&data, "2025-01-01", "2025-01-07");
        assert_eq!(out.total, 3);
        assert_eq!(
            out.by_tag,
            vec![
                TagCount {
                    tag: "A".to_string(),
                    count: 2
                },
                TagCount {
                    tag: "B".to_string(),
                    count: 1
                }
            ]
        );
    }

    /// `compute_today_stats`：当指定日期不存在时应返回 0 与空分组。
    #[test]
    fn compute_today_stats_returns_zero_when_missing_day() {
        let data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: vec![record("A", Phase::Work)],
            }],
            ..AppData::default()
        };

        let out = compute_today_stats(&data, "2025-01-02");
        assert_eq!(out.total, 0);
        assert!(out.by_tag.is_empty());
    }

    /// `compute_week_stats`：应包含闭区间边界日期（from/to 当天）。
    #[test]
    fn compute_week_stats_includes_boundary_days() {
        let data = AppData {
            history: vec![
                HistoryDay {
                    date: "2025-01-01".to_string(),
                    records: vec![record("A", Phase::Work)],
                },
                HistoryDay {
                    date: "2025-01-07".to_string(),
                    records: vec![record("B", Phase::Work)],
                },
                HistoryDay {
                    date: "2025-01-08".to_string(),
                    records: vec![record("C", Phase::Work)],
                },
            ],
            ..AppData::default()
        };

        let out = compute_week_stats(&data, "2025-01-01", "2025-01-07");
        assert_eq!(out.total, 2);
        assert_eq!(
            out.by_tag,
            vec![
                TagCount {
                    tag: "A".to_string(),
                    count: 1
                },
                TagCount {
                    tag: "B".to_string(),
                    count: 1
                }
            ]
        );
    }
}
