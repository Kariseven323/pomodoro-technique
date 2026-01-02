//! 应用路径解析：提供“统一入口”的根目录、数据目录与文件路径工具。

use std::path::PathBuf;

#[cfg(not(windows))]
use tauri::Manager as _;

use crate::app_data::STORE_FILE_NAME;
use crate::errors::{AppError, AppResult};

/// 获取统一入口的根目录。
///
/// - Windows：`%APPDATA%/pomodoro-technique/`
/// - 其他平台：`app_data_dir/`（Tauri 标准应用数据目录）
#[cfg(windows)]
pub fn app_root_dir<R: tauri::Runtime>(_app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| AppError::Invariant("无法读取环境变量 APPDATA".to_string()))?;
    Ok(PathBuf::from(appdata).join("pomodoro-technique"))
}

/// 获取统一入口的根目录。
///
/// - Windows：`%APPDATA%/pomodoro-technique/`
/// - 其他平台：`app_data_dir/`（Tauri 标准应用数据目录）
#[cfg(not(windows))]
pub fn app_root_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|_| AppError::Invariant("无法解析应用数据目录（app_data_dir）".to_string()))
}

/// 获取应用数据目录（位于统一入口根目录下的 `data/`）。
pub fn app_data_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_root_dir(app)?.join("data"))
}

/// 获取 store 文件的最终落盘路径（完整路径）。
pub fn store_file_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join(STORE_FILE_NAME))
}

/// 获取应用日志目录（位于统一入口根目录下的 `logs/`）。
pub fn app_log_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_root_dir(app)?.join("logs"))
}

/// 获取音频目录（位于统一入口根目录下的 `audio/`）。
///
/// PRD v4：自定义音频导入后会被复制到该目录下。
pub fn app_audio_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_root_dir(app)?.join("audio"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `app_root_dir`：非 Windows 下应等同于 Tauri 的 `app_data_dir`。
    #[test]
    #[cfg(not(windows))]
    fn app_root_dir_matches_tauri_app_data_dir_on_non_windows() {
        use tauri::Manager as _;

        let app = tauri::test::mock_app();
        let expected = app.handle().path().app_data_dir().unwrap();
        let out = app_root_dir(app.handle()).unwrap();
        assert_eq!(out, expected);
    }

    /// `app_data_dir/store_file_path/app_log_dir`：应在统一入口根目录下拼接子路径。
    #[test]
    fn derived_paths_join_under_root() {
        let app = tauri::test::mock_app();
        let root = app_root_dir(app.handle()).unwrap();

        assert_eq!(app_data_dir(app.handle()).unwrap(), root.join("data"));
        assert_eq!(
            store_file_path(app.handle()).unwrap(),
            root.join("data").join(STORE_FILE_NAME)
        );
        assert_eq!(app_log_dir(app.handle()).unwrap(), root.join("logs"));
        assert_eq!(app_audio_dir(app.handle()).unwrap(), root.join("audio"));
    }
}
