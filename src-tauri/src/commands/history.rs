//! 历史相关命令：查询历史、编辑备注等。

use crate::app_data::{DateRange, HistoryDay, HistoryRecord};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;
use super::validation::{history_for_ui, history_for_ui_mut, validate_date_range, validate_ymd};

/// 获取指定日期范围内的历史记录（按日分组）。
#[tauri::command]
pub fn get_history(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<HistoryDay>, String> {
    to_ipc_result(get_history_impl(&state, &range))
}

/// 获取历史的内部实现：校验日期范围后按 `YYYY-MM-DD` 字符串过滤（闭区间）。
pub(crate) fn get_history_impl(state: &AppState, range: &DateRange) -> AppResult<Vec<HistoryDay>> {
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

/// 设置某条历史记录的备注（用于“完成后填写”与“历史中编辑”）。
#[tauri::command]
pub fn set_history_remark(
    state: tauri::State<'_, AppState>,
    date: String,
    record_index: usize,
    remark: String,
) -> Result<HistoryRecord, String> {
    to_ipc_result(set_history_remark_impl(&state, date, record_index, remark))
}

/// 设置备注的内部实现：按日期 + 索引定位并持久化。
fn set_history_remark_impl(
    state: &AppState,
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
