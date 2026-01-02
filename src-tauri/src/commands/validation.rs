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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::AppData;

    /// `normalize_name`：应 trim 并转为小写，用于忽略大小写比较。
    #[test]
    fn normalize_name_trims_and_lowercases() {
        assert_eq!(normalize_name(" WeChat.EXE "), "wechat.exe");
    }

    /// `validate_blacklist_items`：空列表应通过。
    #[test]
    fn validate_blacklist_items_accepts_empty() {
        assert!(validate_blacklist_items(&[]).is_ok());
    }

    /// `validate_blacklist_items`：名称与显示名不能为空。
    #[test]
    fn validate_blacklist_items_rejects_blank_fields() {
        let err = validate_blacklist_items(&[BlacklistItem {
            name: "   ".to_string(),
            display_name: "微信".to_string(),
        }])
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let err = validate_blacklist_items(&[BlacklistItem {
            name: "WeChat.exe".to_string(),
            display_name: "   ".to_string(),
        }])
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `validate_blacklist_items`：应拒绝重复进程名（忽略大小写）。
    #[test]
    fn validate_blacklist_items_rejects_duplicates_case_insensitive() {
        let err = validate_blacklist_items(&[
            BlacklistItem {
                name: "WeChat.exe".to_string(),
                display_name: "微信".to_string(),
            },
            BlacklistItem {
                name: "wechat.exe".to_string(),
                display_name: "微信".to_string(),
            },
        ])
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `validate_ymd`：合法日期应通过，非法格式应失败。
    #[test]
    fn validate_ymd_accepts_and_rejects() {
        assert!(validate_ymd("2025-01-01").is_ok());
        assert!(matches!(validate_ymd("2025/01/01"), Err(AppError::Validation(_))));
    }

    /// `validate_date_range`：应校验格式并确保 from <= to。
    #[test]
    fn validate_date_range_validates_order_and_format() {
        assert!(validate_date_range(&DateRange {
            from: "2025-01-01".to_string(),
            to: "2025-01-07".to_string(),
        })
        .is_ok());

        assert!(matches!(
            validate_date_range(&DateRange {
                from: "2025-01-08".to_string(),
                to: "2025-01-07".to_string(),
            }),
            Err(AppError::Validation(_))
        ));

        assert!(matches!(
            validate_date_range(&DateRange {
                from: "bad".to_string(),
                to: "2025-01-07".to_string(),
            }),
            Err(AppError::Validation(_))
        ));
    }

    /// `history_for_ui`：开发环境下若存在 `history_dev`，应优先返回它。
    #[test]
    fn history_for_ui_prefers_history_dev_in_debug() {
        let data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: Vec::new(),
            }],
            history_dev: vec![HistoryDay {
                date: "2099-01-01".to_string(),
                records: Vec::new(),
            }],
            ..AppData::default()
        };

        let out = history_for_ui(&data);
        assert_eq!(out[0].date, "2099-01-01");
    }

    /// `history_for_ui`：当 `history_dev` 为空时应回退到 `history`。
    #[test]
    fn history_for_ui_falls_back_to_history() {
        let data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: Vec::new(),
            }],
            history_dev: Vec::new(),
            ..AppData::default()
        };

        let out = history_for_ui(&data);
        assert_eq!(out[0].date, "2025-01-01");
    }

    /// `history_for_ui_mut`：当 `history_dev` 为空时应回退到 `history`。
    #[test]
    fn history_for_ui_mut_falls_back_to_history() {
        let mut data = AppData {
            history: vec![HistoryDay {
                date: "2025-01-01".to_string(),
                records: Vec::new(),
            }],
            history_dev: Vec::new(),
            ..AppData::default()
        };

        history_for_ui_mut(&mut data).push(HistoryDay {
            date: "2025-01-02".to_string(),
            records: Vec::new(),
        });
        assert_eq!(data.history.len(), 2);
        assert!(data.history_dev.is_empty());
    }
}
