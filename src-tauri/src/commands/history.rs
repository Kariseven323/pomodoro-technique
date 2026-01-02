//! 历史相关命令：查询历史、编辑备注等。

use crate::app_data::{DateRange, HistoryDay, HistoryRecord};
use crate::errors::{AppError, AppResult};

use super::state_like::CommandState;
use super::validation::{history_for_ui, history_for_ui_mut, validate_date_range, validate_ymd};

/// 获取历史的内部实现：校验日期范围后按 `YYYY-MM-DD` 字符串过滤（闭区间）。
pub(crate) fn get_history_impl<S: CommandState>(state: &S, range: &DateRange) -> AppResult<Vec<HistoryDay>> {
    validate_date_range(range)?;

    let data = state.data_snapshot();
    let mut out: Vec<HistoryDay> = history_for_ui(&data)
        .iter()
        .filter(|d| d.date >= range.from && d.date <= range.to)
        .cloned()
        .collect();

    // 让 UI 的“默认本周”更自然：按日期倒序展示。
    out.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(out)
}

/// 设置备注的内部实现：按日期 + 索引定位并持久化。
pub(crate) fn set_history_remark_impl<S: CommandState>(
    state: &S,
    date: String,
    record_index: usize,
    remark: String,
) -> AppResult<HistoryRecord> {
    let date = date.trim().to_string();
    validate_ymd(&date)?;

    let remark = remark.trim().to_string();
    state.update_data(|data| {
        let list = history_for_ui_mut(data);
        let Some(day) = list.iter_mut().find(|d| d.date == date) else {
            return Err(AppError::Validation("找不到指定日期的历史记录".to_string()));
        };
        if record_index >= day.records.len() {
            return Err(AppError::Validation("历史记录索引超出范围".to_string()));
        }
        day.records[record_index].remark = remark.clone();
        Ok(())
    })?;

    tracing::info!(target: "storage", "更新历史备注：date={} index={}", date, record_index);
    let data = state.data_snapshot();
    let day = history_for_ui(&data)
        .iter()
        .find(|d| d.date == date)
        .ok_or_else(|| AppError::Invariant("写入后读取历史失败".to_string()))?;
    Ok(day.records[record_index].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::{AppData, Phase};
    use crate::commands::state_like::CommandState;
    use crate::commands::state_like::TestState;

    /// `get_history_impl`：应按闭区间筛选，并按日期倒序返回。
    #[test]
    fn get_history_filters_and_sorts_desc() {
        let data = AppData {
            history: vec![
                HistoryDay {
                    date: "2025-01-01".to_string(),
                    records: Vec::new(),
                },
                HistoryDay {
                    date: "2025-01-03".to_string(),
                    records: Vec::new(),
                },
            ],
            history_dev: vec![
                HistoryDay {
                    date: "2025-01-02".to_string(),
                    records: Vec::new(),
                },
                HistoryDay {
                    date: "2025-01-04".to_string(),
                    records: Vec::new(),
                },
            ],
            ..AppData::default()
        };
        let state = TestState::new(data);

        let out = get_history_impl(
            &state,
            &DateRange {
                from: "2025-01-02".to_string(),
                to: "2025-01-04".to_string(),
            },
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].date, "2025-01-04");
        assert_eq!(out[1].date, "2025-01-02");
    }

    /// `get_history_impl`：非法日期范围应返回校验错误。
    #[test]
    fn get_history_rejects_invalid_range() {
        let state = TestState::new(AppData::default());
        let err = get_history_impl(
            &state,
            &DateRange {
                from: "2025-01-03".to_string(),
                to: "2025-01-01".to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `set_history_remark_impl`：应更新指定记录的备注并返回更新后的记录。
    #[test]
    fn set_history_remark_updates_record() {
        let data = AppData {
            history_dev: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: vec![HistoryRecord {
                    tag: "学习".to_string(),
                    start_time: "09:00".to_string(),
                    end_time: Some("09:25".to_string()),
                    duration: 25,
                    phase: Phase::Work,
                    remark: String::new(),
                }],
            }],
            ..AppData::default()
        };
        let state = TestState::new(data);

        let updated =
            set_history_remark_impl(&state, "2025-01-01".to_string(), 0, " OK ".to_string())
                .unwrap();
        assert_eq!(updated.remark, "OK");

        let snapshot = state.data_snapshot();
        let day = snapshot
            .history_dev
            .iter()
            .find(|d| d.date == "2025-01-01")
            .unwrap();
        assert_eq!(day.records[0].remark, "OK");
    }

    /// `set_history_remark_impl`：不存在日期或索引越界应返回校验错误。
    #[test]
    fn set_history_remark_rejects_missing_day_or_out_of_range() {
        let state = TestState::new(AppData::default());

        let err = set_history_remark_impl(&state, "2025-01-01".to_string(), 0, "x".to_string())
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let data = AppData {
            history_dev: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: Vec::new(),
            }],
            ..AppData::default()
        };
        let state = TestState::new(data);
        let err = set_history_remark_impl(&state, "2025-01-01".to_string(), 1, "x".to_string())
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `set_history_remark_impl`：日期格式错误应返回校验错误。
    #[test]
    fn set_history_remark_rejects_invalid_date_format() {
        let state = TestState::new(AppData::default());
        let err = set_history_remark_impl(&state, "2025/01/01".to_string(), 0, "x".to_string())
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }
}
