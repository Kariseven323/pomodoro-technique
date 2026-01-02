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
