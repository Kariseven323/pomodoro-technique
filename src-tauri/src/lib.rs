//! 番茄钟应用（Tauri 2）后端入口与命令注册。

mod app_data;
mod commands;
mod errors;
mod processes;
mod state;
mod timer;
mod tray;

use std::time::Duration;

use tauri::Manager as _;
use tauri_plugin_store::StoreExt;

use crate::app_data::{AppData, STORE_KEY, STORE_PATH};
use crate::errors::AppResult;
use crate::state::AppState;
use crate::timer::spawn_timer_task;
use crate::tray::setup_tray;

/// 应用运行入口（由 `src-tauri/src/main.rs` 调用）。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let store = app
                .store_builder(STORE_PATH)
                .auto_save(Duration::from_millis(0))
                .build()?;

            let data = load_or_init_app_data(&store)?;

            app.manage(AppState::new(app.handle().clone(), store, data));

            setup_tray(app)?;
            setup_window_close_to_tray(app)?;
            spawn_timer_task(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::get_store_paths,
            commands::open_store_dir,
            commands::update_settings,
            commands::set_current_tag,
            commands::add_tag,
            commands::set_blacklist,
            commands::list_processes,
            commands::timer_start,
            commands::timer_pause,
            commands::timer_reset,
            commands::timer_skip,
            commands::restart_as_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 从 store 中加载 `AppData`；若为空则写入默认值并返回。
fn load_or_init_app_data(
    store: &tauri_plugin_store::Store<tauri::Wry>,
) -> AppResult<AppData> {
    if let Some(value) = store.get(STORE_KEY) {
        let data: AppData = serde_json::from_value(value)?;
        return Ok(data);
    }

    let data = AppData::default();
    store.set(STORE_KEY, serde_json::to_value(&data)?);
    store.save()?;
    Ok(data)
}

/// 将窗口关闭行为改为“隐藏到托盘”（满足 PRD 的“最小化到托盘”）。
fn setup_window_close_to_tray(app: &mut tauri::App) -> AppResult<()> {
    use tauri::Manager as _;
    use tauri::WindowEvent;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| errors::AppError::Invariant("主窗口 `main` 不存在".to_string()))?;

    let window_for_event = window.clone();
    window.on_window_event(move |event| {
        match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window_for_event.hide();
            }
            _ => {}
        }
    });

    Ok(())
}
