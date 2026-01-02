//! 日志相关命令：打开日志目录、前端诊断日志桥接等。

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

/// 将前端上报的日志内容做基础清洗与截断，避免单条日志过长影响可读性。
fn sanitize_frontend_log_message(message: &str) -> String {
    const MAX_CHARS: usize = 4_000;
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }
    let mut out = String::with_capacity(MAX_CHARS + 16);
    for (i, ch) in trimmed.chars().enumerate() {
        if i >= MAX_CHARS {
            break;
        }
        out.push(ch);
    }
    out.push_str("…(truncated)");
    out
}

/// 前端日志桥接的内部实现：release 也允许写入（忽略 debug，避免将该入口用于高频噪声日志）。
#[cfg(not(debug_assertions))]
pub(crate) fn frontend_log_impl(level: &str, message: &str) -> AppResult<bool> {
    let lvl = level.trim().to_lowercase();
    let msg = sanitize_frontend_log_message(message);
    match lvl.as_str() {
        "warn" | "warning" => tracing::warn!(target: "frontend", "{msg}"),
        "error" => tracing::error!(target: "frontend", "{msg}"),
        "debug" => {
            // release 下默认忽略 debug，避免高频噪声日志写入文件。
        }
        _ => tracing::info!(target: "frontend", "{msg}"),
    }
    Ok(true)
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
