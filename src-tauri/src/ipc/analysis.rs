//! 分析相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::app_data::DateRange;
use crate::commands::analysis::get_focus_analysis_impl;
use crate::commands::common::to_ipc_result;
use crate::errors::AppResult;
use crate::state::AppState;

/// 获取指定范围的专注分析数据（用于“专注时段分析”图表/摘要）。
#[tauri::command]
pub fn get_focus_analysis(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<crate::analysis::FocusAnalysis, String> {
    to_ipc_result(get_focus_analysis_ipc_impl(&*state, &range))
}

/// IPC 内部实现：复用 `commands::analysis` 的可测试实现。
fn get_focus_analysis_ipc_impl(state: &AppState, range: &DateRange) -> AppResult<crate::analysis::FocusAnalysis> {
    get_focus_analysis_impl(state, range)
}

