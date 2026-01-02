//! 历史相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::app_data::{DateRange, HistoryRecord};
use crate::commands::common::to_ipc_result;
use crate::commands::history::{get_history_impl, set_history_remark_impl};
use crate::state::AppState;

/// 获取历史记录（按日期范围筛选；用于历史列表与统计）。
#[tauri::command]
pub fn get_history(state: tauri::State<'_, AppState>, range: DateRange) -> Result<Vec<crate::app_data::HistoryDay>, String> {
    to_ipc_result(get_history_impl(&*state, &range))
}

/// 修改指定历史记录备注（用于工作完成后补充备注）。
#[tauri::command]
pub fn set_history_remark(
    state: tauri::State<'_, AppState>,
    date: String,
    record_index: usize,
    remark: String,
) -> Result<HistoryRecord, String> {
    to_ipc_result(set_history_remark_impl(&*state, date, record_index, remark))
}
