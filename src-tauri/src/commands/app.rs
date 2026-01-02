//! 应用级命令：快照、数据目录等。

use crate::app_paths;
use crate::errors::AppResult;

use super::types::StorePaths;

/// 获取应用数据根目录路径的内部实现（统一入口）。
pub(crate) fn get_store_paths_impl<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<StorePaths> {
    let store_dir = app_paths::app_root_dir(app)?;
    Ok(StorePaths {
        store_dir_path: store_dir.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `get_store_paths_impl`：应返回与 `app_paths::app_root_dir` 一致的路径字符串。
    #[test]
    fn get_store_paths_returns_root_dir_path() {
        let app = tauri::test::mock_app();
        let expected = app_paths::app_root_dir(app.handle())
            .unwrap()
            .to_string_lossy()
            .to_string();

        let out = get_store_paths_impl(app.handle()).unwrap();
        assert_eq!(out.store_dir_path, expected);
    }
}
