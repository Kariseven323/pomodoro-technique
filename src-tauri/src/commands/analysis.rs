//! 分析相关命令：专注时段分析。

use crate::analysis::FocusAnalysis;
use crate::app_data::DateRange;
use crate::errors::AppResult;

use super::state_like::CommandState;
use super::validation::{history_for_ui, validate_date_range};

/// 获取专注分析的内部实现。
pub(crate) fn get_focus_analysis_impl<S: CommandState>(
    state: &S,
    range: &DateRange,
) -> AppResult<FocusAnalysis> {
    validate_date_range(range)?;
    let data = state.data_snapshot();
    crate::analysis::get_focus_analysis(history_for_ui(&data), range)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::{AppData, HistoryDay, HistoryRecord, Phase};
    use crate::commands::state_like::TestState;

    /// `get_focus_analysis_impl`：应校验日期范围，并在开发环境优先使用 `history_dev`。
    #[test]
    fn get_focus_analysis_prefers_history_dev_and_validates_range() {
        let data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: vec![HistoryRecord {
                    tag: "A".to_string(),
                    start_time: "09:00".to_string(),
                    end_time: None,
                    duration: 25,
                    phase: Phase::Work,
                    remark: String::new(),
                }],
            }],
            history_dev: vec![HistoryDay {
                date: "2025-01-02".to_string(),
                records: vec![HistoryRecord {
                    tag: "B".to_string(),
                    start_time: "10:00".to_string(),
                    end_time: None,
                    duration: 30,
                    phase: Phase::Work,
                    remark: String::new(),
                }],
            }],
            ..AppData::default()
        };
        let state = TestState::new(data);

        let out = get_focus_analysis_impl(
            &state,
            &DateRange {
                from: "2025-01-02".to_string(),
                to: "2025-01-02".to_string(),
            },
        )
        .unwrap();
        assert_eq!(out.hourly_counts[10], 1);
        assert!(out.tag_efficiency.iter().any(|t| t.tag == "B"));

        let err = get_focus_analysis_impl(
            &state,
            &DateRange {
                from: "2025-01-03".to_string(),
                to: "2025-01-01".to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, crate::errors::AppError::Validation(_)));
    }
}
