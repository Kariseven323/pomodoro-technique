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
