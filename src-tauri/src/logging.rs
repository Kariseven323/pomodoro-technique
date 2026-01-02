//! 应用日志系统：基于 `tracing` 将日志写入文件，并提供日志目录解析工具。

use std::path::{Path, PathBuf};

use tracing_subscriber::prelude::*;

use crate::app_paths;
use crate::errors::{AppError, AppResult};

/// 日志文件名前缀（配合 `tracing_appender` 生成 `pomodoro-YYYY-MM-DD.log`）。
const LOG_PREFIX: &str = "pomodoro-";

/// 日志文件名后缀。
const LOG_SUFFIX: &str = ".log";

/// 单日志文件建议上限（字节；PRD v2：10MB）。
pub const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// 日志文件保留数量（PRD v2：保留最近 7 个）。
pub const MAX_LOG_FILES: usize = 7;

/// 获取日志目录（统一入口：`root/logs/`）。
pub fn log_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    app_paths::app_log_dir(app)
}

/// 初始化文件日志（daily rolling + 保留最近若干文件）。
pub fn init_logging<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<()> {
    let dir = log_dir(app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Invariant(format!("创建日志目录失败：{e}")))?;

    cleanup_old_logs(&dir, MAX_LOG_FILES)?;

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_PREFIX)
        .filename_suffix(LOG_SUFFIX)
        .max_log_files(MAX_LOG_FILES)
        .build(&dir)
        .map_err(|e| AppError::Invariant(format!("初始化日志写入器失败：{e}")))?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 将 guard 泄漏为全局生命周期：保证应用运行期间日志线程不被提前 drop。
    // 该 guard 仅在进程退出时回收，符合桌面应用预期。
    std::mem::forget(_guard);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // 注意：测试环境可能已提前初始化 tracing（例如覆盖率/单测辅助），此时无需重复设置全局 subscriber。
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(non_blocking)
                .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                    "%Y-%m-%d %H:%M:%S%.3f".to_string(),
                )),
        )
        .try_init();

    tracing::info!(target: "system", "日志系统已初始化，目录={}", dir.to_string_lossy());
    Ok(())
}

/// 清理旧日志：按文件名排序保留最近 `max_files` 个。
fn cleanup_old_logs(dir: &Path, max_files: usize) -> AppResult<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in
        std::fs::read_dir(dir).map_err(|e| AppError::Invariant(format!("读取日志目录失败：{e}")))?
    {
        let entry = entry.map_err(|e| AppError::Invariant(format!("读取日志目录条目失败：{e}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with(LOG_PREFIX) && name.ends_with(LOG_SUFFIX) {
            // PRD v2：单文件上限 10MB（这里在启动时对历史日志做一次截断，避免无限增长）。
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > MAX_LOG_FILE_BYTES {
                    let _ = std::fs::OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .and_then(|f| f.set_len(MAX_LOG_FILE_BYTES));
                }
            }
            files.push(path);
        }
    }

    files.sort();
    if files.len() <= max_files {
        return Ok(());
    }

    let remove_count = files.len() - max_files;
    for path in files.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 生成一个测试用日志文件路径（符合前缀/后缀规则）。
    fn log_file_path(dir: &std::path::Path, ymd: &str) -> std::path::PathBuf {
        dir.join(format!("{LOG_PREFIX}{ymd}{LOG_SUFFIX}"))
    }

    /// `cleanup_old_logs`：应仅保留最近 `max_files` 个日志文件（按文件名排序）。
    #[test]
    fn cleanup_old_logs_keeps_latest_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();

        std::fs::write(log_file_path(dir_path, "2025-01-01"), "a").unwrap();
        std::fs::write(log_file_path(dir_path, "2025-01-02"), "b").unwrap();
        std::fs::write(log_file_path(dir_path, "2025-01-03"), "c").unwrap();
        std::fs::write(dir_path.join("other.txt"), "x").unwrap();

        cleanup_old_logs(dir_path, 2).unwrap();

        assert!(!log_file_path(dir_path, "2025-01-01").exists());
        assert!(log_file_path(dir_path, "2025-01-02").exists());
        assert!(log_file_path(dir_path, "2025-01-03").exists());
        assert!(dir_path.join("other.txt").exists());
    }

    /// `cleanup_old_logs`：单文件超过上限时应截断到 `MAX_LOG_FILE_BYTES`。
    #[test]
    fn cleanup_old_logs_truncates_large_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();
        let file = log_file_path(dir_path, "2025-01-01");

        // 使用 set_len 快速制造一个“大文件”，避免写入大量内容。
        let f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file)
            .unwrap();
        f.set_len(MAX_LOG_FILE_BYTES + 1).unwrap();

        cleanup_old_logs(dir_path, 7).unwrap();

        let size = std::fs::metadata(&file).unwrap().len();
        assert_eq!(size, MAX_LOG_FILE_BYTES);
    }

    /// `cleanup_old_logs`：应跳过非文件条目与非 UTF-8 文件名（不应报错）。
    #[test]
    fn cleanup_old_logs_skips_non_file_and_non_utf8_names() {
        use std::os::unix::ffi::OsStringExt as _;

        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();

        // 非文件条目：目录。
        std::fs::create_dir_all(dir_path.join("subdir")).unwrap();

        // 非 UTF-8 文件名：用非法字节构造。
        let invalid_name = std::ffi::OsString::from_vec(vec![0xFF, 0xFE, b'.', b'l', b'o', b'g']);
        let invalid_path = dir_path.join(invalid_name);
        std::fs::write(&invalid_path, "x").unwrap();

        // 正常日志文件：确保函数仍会正常处理其它文件。
        std::fs::write(log_file_path(dir_path, "2025-01-01"), "a").unwrap();

        cleanup_old_logs(dir_path, 7).unwrap();
        assert!(log_file_path(dir_path, "2025-01-01").exists());
    }

    /// `init_logging`：应可在测试环境初始化（若已初始化也应保持幂等且返回 Ok）。
    #[test]
    fn init_logging_is_idempotent_in_tests() {
        let app = tauri::test::mock_app();
        init_logging(app.handle()).unwrap();
        init_logging(app.handle()).unwrap();
    }
}
