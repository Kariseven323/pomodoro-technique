//! 命令模块通用工具：错误转换与 IPC 适配。

use crate::errors::{AppError, AppResult};

/// 将内部 `AppResult<T>` 转为可被 Tauri IPC 接受的 `Result<T, String>`。
pub(crate) fn to_ipc_result<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(app_error_to_string)
}

/// 将 `AppError` 转为前端可展示的错误字符串。
fn app_error_to_string(err: AppError) -> String {
    match err {
        AppError::BlacklistLocked => "专注期内禁止移除黑名单进程".to_string(),
        AppError::Validation(msg) => msg,
        AppError::UnsupportedPlatform(msg) => msg,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `to_ipc_result`：应将 `AppError::BlacklistLocked` 映射为前端友好提示。
    #[test]
    fn to_ipc_result_maps_blacklist_locked() {
        let out: Result<(), String> = to_ipc_result(Err(AppError::BlacklistLocked));
        assert_eq!(out.unwrap_err(), "专注期内禁止移除黑名单进程");
    }

    /// `to_ipc_result`：应透传校验错误与不支持平台错误的消息内容。
    #[test]
    fn to_ipc_result_passthrough_validation_and_unsupported() {
        let out: Result<(), String> = to_ipc_result(Err(AppError::Validation("x".to_string())));
        assert_eq!(out.unwrap_err(), "x");

        let out: Result<(), String> =
            to_ipc_result(Err(AppError::UnsupportedPlatform("y".to_string())));
        assert_eq!(out.unwrap_err(), "y");
    }

    /// `to_ipc_result`：其它错误应回退到 `Display` 文案。
    #[test]
    fn to_ipc_result_falls_back_to_display_for_other_errors() {
        let out: Result<(), String> = to_ipc_result(Err(AppError::Invariant("boom".to_string())));
        assert!(out.unwrap_err().contains("boom"));
    }
}
