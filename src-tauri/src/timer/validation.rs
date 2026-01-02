//! 设置参数校验（用于后端命令统一验证）。

use crate::app_data::Settings;
use crate::errors::{AppError, AppResult};

/// 校验并规范化设置参数（同时满足 PRD 的范围约束）。
pub fn validate_settings(settings: &Settings) -> AppResult<()> {
    if !(1..=60).contains(&settings.pomodoro) {
        return Err(AppError::Validation("番茄时长需在 1-60 分钟".to_string()));
    }
    if !(1..=30).contains(&settings.short_break) {
        return Err(AppError::Validation("短休息需在 1-30 分钟".to_string()));
    }
    if !(1..=60).contains(&settings.long_break) {
        return Err(AppError::Validation("长休息需在 1-60 分钟".to_string()));
    }
    if !(1..=10).contains(&settings.long_break_interval) {
        return Err(AppError::Validation(
            "长休息间隔需在 1-10 个番茄".to_string(),
        ));
    }
    if !(1..=20).contains(&settings.auto_continue_pomodoros) {
        return Err(AppError::Validation(
            "连续番茄数量需在 1-20 个番茄".to_string(),
        ));
    }
    if settings.daily_goal > 1000 {
        return Err(AppError::Validation("每日目标建议不超过 1000".to_string()));
    }
    if settings.weekly_goal > 10000 {
        return Err(AppError::Validation("每周目标建议不超过 10000".to_string()));
    }
    if settings.audio.volume > 100 {
        return Err(AppError::Validation("音效音量需在 0-100".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 校验：合法边界应通过。
    #[test]
    fn validate_settings_accepts_valid_ranges() {
        let settings = Settings {
            pomodoro: 1,
            short_break: 1,
            long_break: 1,
            long_break_interval: 1,
            auto_continue_enabled: true,
            auto_continue_pomodoros: 1,
            daily_goal: 0,
            weekly_goal: 0,
            always_on_top: false,
            ..Settings::default()
        };
        assert!(validate_settings(&settings).is_ok());
    }

    /// 校验：非法范围应返回 `Validation` 错误。
    #[test]
    fn validate_settings_rejects_out_of_range() {
        assert!(matches!(
            validate_settings(&Settings {
                pomodoro: 0,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                pomodoro: 61,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                short_break: 0,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                short_break: 31,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                long_break: 0,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                long_break: 61,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                long_break_interval: 0,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                long_break_interval: 11,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
    }

    /// 校验：连续番茄数量超出范围应失败。
    #[test]
    fn validate_settings_rejects_auto_continue_pomodoros_out_of_range() {
        assert!(matches!(
            validate_settings(&Settings {
                auto_continue_pomodoros: 0,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                auto_continue_pomodoros: 21,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
    }

    /// 校验：每日/每周目标过大应失败（用于防御性约束）。
    #[test]
    fn validate_settings_rejects_excessive_goals() {
        assert!(matches!(
            validate_settings(&Settings {
                daily_goal: 1001,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_settings(&Settings {
                weekly_goal: 10001,
                ..Settings::default()
            }),
            Err(AppError::Validation(_))
        ));
    }
}
