//! 分析相关命令：专注时段分析。

use crate::analysis::FocusAnalysis;
use crate::app_data::DateRange;
use crate::errors::AppResult;
use crate::state::AppState;

use super::common::to_ipc_result;
use super::validation::{history_for_ui, validate_date_range};

/// 获取指定范围的专注分析数据（用于“专注时段分析”图表/摘要）。
#[tauri::command]
pub fn get_focus_analysis(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<FocusAnalysis, String> {
    to_ipc_result(get_focus_analysis_impl(&state, &range))
}

/// 获取专注分析的内部实现。
fn get_focus_analysis_impl(state: &AppState, range: &DateRange) -> AppResult<FocusAnalysis> {
    validate_date_range(range)?;
    let data = state.data_snapshot();
    crate::analysis::get_focus_analysis(history_for_ui(&data), range)
}
