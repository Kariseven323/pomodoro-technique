//! 统一错误类型与 `Result` 别名。

use thiserror::Error;

/// 应用内部通用错误。
#[derive(Debug, Error)]
pub enum AppError {
    /// JSON 序列化/反序列化失败。
    #[error("JSON 处理失败：{0}")]
    Json(#[from] serde_json::Error),

    /// Store 插件失败。
    #[error("Store 失败：{0}")]
    Store(#[from] tauri_plugin_store::Error),

    /// Tauri 框架错误。
    #[error("Tauri 运行失败：{0}")]
    Tauri(#[from] tauri::Error),

    /// 系统通知失败。
    #[error("通知失败：{0}")]
    Notification(#[from] tauri_plugin_notification::Error),

    /// 参数校验失败。
    #[error("参数不合法：{0}")]
    Validation(String),

    /// 黑名单在专注期被锁定，禁止移除。
    #[error("专注期内禁止移除黑名单进程")]
    BlacklistLocked,

    /// 平台不支持。
    #[error("当前平台不支持：{0}")]
    #[cfg_attr(windows, allow(dead_code))]
    UnsupportedPlatform(String),

    /// 内部不变量被破坏（逻辑错误）。
    #[error("内部错误：{0}")]
    Invariant(String),

    /// 终止进程失败（通常是权限不足，仅 Windows 使用）。
    #[error("终止进程失败：{0}")]
    #[cfg(windows)]
    KillFailed(String),
}

/// 应用内部 `Result` 统一别名。
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// `AppError::Json`：Display 应包含“JSON 处理失败”前缀。
    #[test]
    fn app_error_display_json() {
        let err = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let app_err = AppError::from(err);
        assert!(app_err.to_string().contains("JSON 处理失败："));
    }

    /// `AppError::Store`：Display 应包含“Store 失败”前缀。
    #[test]
    fn app_error_display_store() {
        let err =
            tauri_plugin_store::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "boom"));
        let app_err = AppError::from(err);
        assert!(app_err.to_string().contains("Store 失败："));
        assert!(app_err.to_string().contains("boom"));
    }

    /// `AppError::Tauri`：Display 应包含“Tauri 运行失败”前缀。
    #[test]
    fn app_error_display_tauri() {
        let err = tauri::Error::AssetNotFound("missing".to_string());
        let app_err = AppError::from(err);
        assert!(app_err.to_string().contains("Tauri 运行失败："));
        assert!(app_err.to_string().contains("asset not found"));
    }

    /// `AppError::Notification`：Display 应包含“通知失败”前缀。
    #[test]
    fn app_error_display_notification() {
        let err = tauri_plugin_notification::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "notify",
        ));
        let app_err = AppError::from(err);
        assert!(app_err.to_string().contains("通知失败："));
        assert!(app_err.to_string().contains("notify"));
    }

    /// `AppError::Validation`：Display 应包含“参数不合法”前缀与原始信息。
    #[test]
    fn app_error_display_validation() {
        let app_err = AppError::Validation("bad".to_string());
        assert_eq!(app_err.to_string(), "参数不合法：bad");
    }

    /// `AppError::BlacklistLocked`：Display 应为固定文案。
    #[test]
    fn app_error_display_blacklist_locked() {
        let app_err = AppError::BlacklistLocked;
        assert_eq!(app_err.to_string(), "专注期内禁止移除黑名单进程");
    }

    /// `AppError::UnsupportedPlatform`：Display 应包含“不支持”与原因。
    #[test]
    fn app_error_display_unsupported_platform() {
        let app_err = AppError::UnsupportedPlatform("x".to_string());
        assert_eq!(app_err.to_string(), "当前平台不支持：x");
    }

    /// `AppError::Invariant`：Display 应包含“内部错误”前缀。
    #[test]
    fn app_error_display_invariant() {
        let app_err = AppError::Invariant("oops".to_string());
        assert_eq!(app_err.to_string(), "内部错误：oops");
    }

    /// `AppError::KillFailed`：Windows 下 Display 应包含“终止进程失败”前缀。
    #[cfg(windows)]
    #[test]
    fn app_error_display_kill_failed_on_windows() {
        let app_err = AppError::KillFailed("ACCESS_DENIED".to_string());
        assert!(app_err.to_string().contains("终止进程失败："));
        assert!(app_err.to_string().contains("ACCESS_DENIED"));
    }
}
