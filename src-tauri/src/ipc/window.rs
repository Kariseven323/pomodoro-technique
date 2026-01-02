//! 窗口相关 IPC 命令：置顶、迷你模式、退出等（UI 边界）。

use tauri::{LogicalPosition, LogicalSize, Manager as _};

use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use crate::commands::common::to_ipc_result;

/// 设置主窗口置顶状态（并持久化到 settings）。
#[tauri::command]
pub fn set_always_on_top(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    to_ipc_result(set_always_on_top_impl(&app, &state, enabled))
}

/// 设置置顶的内部实现：修改窗口并写入 settings。
fn set_always_on_top_impl(
    app: &tauri::AppHandle,
    state: &AppState,
    enabled: bool,
) -> AppResult<bool> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Invariant("主窗口 `main` 不存在".to_string()))?;
    window.set_always_on_top(enabled)?;

    state.update_data(|data| {
        data.settings.always_on_top = enabled;
        Ok(())
    })?;

    tracing::info!(target: "window", "设置置顶：enabled={}", enabled);
    Ok(true)
}

/// 切换迷你模式：窗口调整为 200x80，仅显示倒计时；再次关闭恢复原尺寸。
#[tauri::command]
pub fn set_mini_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    to_ipc_result(set_mini_mode_impl(&app, &state, enabled))
}

/// 迷你模式内部实现：记录进入前的尺寸/位置，便于恢复。
fn set_mini_mode_impl(app: &tauri::AppHandle, state: &AppState, enabled: bool) -> AppResult<bool> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Invariant("主窗口 `main` 不存在".to_string()))?;

    let snapshot = state.window_mode_snapshot();
    if enabled && !snapshot.mini_mode {
        if let Ok(size) = window.outer_size() {
            let _ = state.update_window_mode(|m| {
                m.prev_size = Some((size.width, size.height));
                Ok(())
            });
        }
        if let Ok(pos) = window.outer_position() {
            let _ = state.update_window_mode(|m| {
                m.prev_position = Some((pos.x, pos.y));
                Ok(())
            });
        }
    }

    if enabled {
        window.set_resizable(false)?;
        window.set_size(LogicalSize::new(200.0, 80.0))?;
        let _ = state.update_window_mode(|m| {
            m.mini_mode = true;
            Ok(())
        });
    } else {
        let snapshot = state.window_mode_snapshot();
        window.set_resizable(false)?;
        if let Some((w, h)) = snapshot.prev_size {
            window.set_size(LogicalSize::new(w as f64, h as f64))?;
        } else {
            window.set_size(LogicalSize::new(420.0, 720.0))?;
        }
        if let Some((x, y)) = snapshot.prev_position {
            window.set_position(LogicalPosition::new(x as f64, y as f64))?;
        }
        let _ = state.update_window_mode(|m| {
            m.mini_mode = false;
            Ok(())
        });
    }

    tracing::info!(target: "window", "切换迷你模式：enabled={}", enabled);
    Ok(true)
}

/// 退出应用（用于迷你模式右键菜单）。
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    to_ipc_result(exit_app_impl(&app, &state))
}

/// 退出应用的内部实现：请求 Tauri 退出。
fn exit_app_impl(app: &tauri::AppHandle, state: &AppState) -> AppResult<bool> {
    let _ = state.record_quit_interruption_before_exit();
    tracing::info!(target: "system", "请求退出应用");
    app.exit(0);
    Ok(true)
}
