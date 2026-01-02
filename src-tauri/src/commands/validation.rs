//! 命令层输入校验与通用数据选择逻辑（避免散落在各个模块中）。

use crate::app_data::{BlacklistItem, DateRange, HistoryDay};
use crate::errors::{AppError, AppResult};

/// 校验黑名单条目：名称不能为空、不得重复（忽略大小写）。
pub(crate) fn validate_blacklist_items(items: &[BlacklistItem]) -> AppResult<()> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    for it in items {
        if it.name.trim().is_empty() {
            return Err(AppError::Validation("黑名单进程名不能为空".to_string()));
        }
        if it.display_name.trim().is_empty() {
            return Err(AppError::Validation("黑名单显示名不能为空".to_string()));
        }
        let key = normalize_name(&it.name);
        if !seen.insert(key) {
            return Err(AppError::Validation("黑名单存在重复进程名".to_string()));
        }
    }
    Ok(())
}

/// 规范化进程名用于比较（Windows 下大小写不敏感）。
pub(crate) fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// 校验日期字符串是否符合 `YYYY-MM-DD`。
pub(crate) fn validate_ymd(date: &str) -> AppResult<()> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;
    Ok(())
}

/// 校验日期范围：格式正确且 `from <= to`。
pub(crate) fn validate_date_range(range: &DateRange) -> AppResult<()> {
    validate_ymd(range.from.trim())?;
    validate_ymd(range.to.trim())?;
    if range.from.trim() > range.to.trim() {
        return Err(AppError::Validation(
            "日期范围不合法：from 不能晚于 to".to_string(),
        ));
    }
    Ok(())
}

/// 选择供“历史页面/导出/分析”使用的历史数据源（开发环境：优先 `history_dev`）。
pub(crate) fn history_for_ui(data: &crate::app_data::AppData) -> &Vec<HistoryDay> {
    if cfg!(debug_assertions) && !data.history_dev.is_empty() {
        &data.history_dev
    } else {
        &data.history
    }
}

/// 选择供“历史备注编辑”使用的可变历史数据源（开发环境：优先 `history_dev`）。
pub(crate) fn history_for_ui_mut(data: &mut crate::app_data::AppData) -> &mut Vec<HistoryDay> {
    if cfg!(debug_assertions) && !data.history_dev.is_empty() {
        &mut data.history_dev
    } else {
        &mut data.history
    }
}
