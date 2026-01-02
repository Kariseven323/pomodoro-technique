//! 日志相关命令：打开日志目录、前端诊断日志桥接等。

#[cfg(not(debug_assertions))]
use crate::errors::AppError;
use crate::errors::AppResult;

/// 前端日志桥接的内部实现：按 level 写入 tracing。
#[cfg(debug_assertions)]
pub(crate) fn frontend_log_impl(level: &str, message: &str) -> AppResult<bool> {
    let lvl = level.trim().to_lowercase();
    match lvl.as_str() {
        "debug" => tracing::debug!(target: "frontend", "{message}"),
        "warn" | "warning" => tracing::warn!(target: "frontend", "{message}"),
        "error" => tracing::error!(target: "frontend", "{message}"),
        _ => tracing::info!(target: "frontend", "{message}"),
    }
    Ok(true)
}

/// 前端日志桥接的内部实现：非开发环境直接拒绝（避免 release 包携带诊断日志入口）。
#[cfg(not(debug_assertions))]
pub(crate) fn frontend_log_impl(_level: &str, _message: &str) -> AppResult<bool> {
    Err(AppError::Validation(
        "仅开发环境可使用前端诊断日志（frontend_log）".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `frontend_log_impl`：应按 level 分支写入，并在开发环境返回 true。
    #[test]
    fn frontend_log_handles_levels() {
        assert!(frontend_log_impl("debug", "a").unwrap());
        assert!(frontend_log_impl("warn", "b").unwrap());
        assert!(frontend_log_impl("warning", "c").unwrap());
        assert!(frontend_log_impl("error", "d").unwrap());
        assert!(frontend_log_impl("info", "e").unwrap());
        assert!(frontend_log_impl("unknown", "f").unwrap());
    }
}
