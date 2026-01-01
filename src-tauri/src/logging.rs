//! 应用日志系统：基于 `tracing` 将日志写入文件，并提供日志目录解析工具。

use std::path::{Path, PathBuf};

use tauri::Manager as _;
use tracing_subscriber::prelude::*;

use crate::errors::{AppError, AppResult};

/// 日志文件名前缀（配合 `tracing_appender` 生成 `pomodoro-YYYY-MM-DD.log`）。
const LOG_PREFIX: &str = "pomodoro-";

/// 日志文件名后缀。
const LOG_SUFFIX: &str = ".log";

/// 单日志文件建议上限（字节；PRD v2：10MB）。
pub const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// 日志文件保留数量（PRD v2：保留最近 7 个）。
pub const MAX_LOG_FILES: usize = 7;

/// 获取日志目录（Windows：`%APPDATA%/pomodoro-technique/logs/`；其他平台：`app_data_dir/logs/`）。
pub fn log_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| AppError::Invariant("无法读取环境变量 APPDATA".to_string()))?;
        return Ok(Path::new(&appdata).join("pomodoro-technique").join("logs"));
    }

    #[cfg(not(windows))]
    {
        let base = app
            .path()
            .app_data_dir()
            .map_err(|_| AppError::Invariant("无法解析应用数据目录（app_data_dir）".to_string()))?;
        Ok(base.join("logs"))
    }
}

/// 初始化文件日志（daily rolling + 保留最近若干文件）。
pub fn init_logging(app: &tauri::AppHandle) -> AppResult<()> {
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

    tracing_subscriber::registry()
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
        .init();

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
