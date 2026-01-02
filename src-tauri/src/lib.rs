//! 番茄钟应用（Tauri 2）后端入口与命令注册。

#![cfg_attr(test, allow(dead_code))]

mod analysis;
mod app_data;
mod app_paths;
mod audio;
mod combo;
mod commands;
mod errors;
mod events;
mod interruptions;
#[cfg(not(test))]
mod ipc;
mod logging;
mod processes;
#[cfg(not(test))]
mod state;
mod timer;
#[cfg(not(test))]
mod tray;
pub mod typegen;
mod window_events;

#[cfg(not(test))]
use std::time::Duration;

#[cfg(not(test))]
use tauri::Manager as _;
#[cfg(not(test))]
use tauri_plugin_store::StoreExt;

#[cfg(not(test))]
use crate::app_data::STORE_FILE_NAME;
#[cfg(not(test))]
use crate::app_data::{AppData, STORE_KEY};
#[cfg(not(test))]
use crate::errors::{AppError, AppResult};

#[cfg(not(test))]
use crate::state::AppState;
#[cfg(not(test))]
use crate::timer::spawn_timer_task;
#[cfg(not(test))]
use crate::tray::setup_tray;

/// 应用运行入口（由 `src-tauri/src/main.rs` 调用）。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[cfg(not(test))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            logging::init_logging(app.handle())?;
            migrate_legacy_store_file(app.handle())?;

            let store_path = app_paths::store_file_path(app.handle())?;
            let store = app
                .store_builder(store_path)
                .auto_save(Duration::from_millis(0))
                .build()?;

            let data = load_or_init_app_data(&store)?;

            app.manage(AppState::new(app.handle().clone(), store, data)?);

            setup_tray(app)?;
            setup_window_close_to_tray(app)?;
            spawn_timer_task(app.handle().clone());

            // PRD v2：启动时应用“窗口置顶”设置。
            if let Some(window) = app.get_webview_window("main") {
                let state = app.state::<AppState>();
                let always_on_top = state.data_snapshot().settings.always_on_top;
                let _ = window.set_always_on_top(always_on_top);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::app::get_app_snapshot,
            ipc::app::get_store_paths,
            ipc::app::open_store_dir,
            ipc::settings::update_settings,
            ipc::settings::set_goals,
            ipc::tags::set_current_tag,
            ipc::tags::add_tag,
            ipc::tags::rename_tag,
            ipc::tags::delete_tag,
            ipc::blacklist::set_blacklist,
            ipc::history::get_history,
            ipc::history::set_history_remark,
            ipc::analysis::get_focus_analysis,
            ipc::audio::audio_list,
            ipc::audio::audio_play,
            ipc::audio::audio_pause,
            ipc::audio::audio_set_volume,
            ipc::audio::audio_import,
            ipc::audio::audio_delete,
            ipc::templates::get_templates,
            ipc::templates::save_template,
            ipc::templates::delete_template,
            ipc::templates::apply_template,
            ipc::window::set_always_on_top,
            ipc::window::set_mini_mode,
            ipc::export::export_history,
            ipc::logging::open_log_dir,
            ipc::logging::frontend_log,
            ipc::debug::debug_generate_history,
            ipc::debug::debug_clear_history,
            ipc::window::exit_app,
            ipc::processes::list_processes,
            ipc::processes::process_icon,
            ipc::timer::timer_start,
            ipc::timer::timer_pause,
            ipc::timer::timer_reset,
            ipc::timer::timer_skip,
            ipc::interruption::record_interruption,
            ipc::interruption::get_interruption_stats,
            ipc::interruption::get_combo,
            ipc::interruption::get_total_pomodoros,
            ipc::processes::restart_as_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 将相对 `BaseDirectory::AppData` 的路径解析为真实磁盘路径（用于兼容迁移）。
#[cfg(not(test))]
fn resolve_path_in_app_data(app: &tauri::AppHandle, path: &str) -> AppResult<std::path::PathBuf> {
    use tauri::path::BaseDirectory;
    use tauri::Manager as _;

    app.path()
        .resolve(path, BaseDirectory::AppData)
        .map_err(|_| AppError::Invariant(format!("无法解析路径（BaseDirectory::AppData）：{path}")))
}

/// 启动迁移：将旧位置/误位置的 store 文件搬到“统一入口”根目录下的 `data/`（仅在新文件不存在时执行）。
#[cfg(not(test))]
fn migrate_legacy_store_file(app: &tauri::AppHandle) -> AppResult<()> {
    let target = app_paths::store_file_path(app)?;
    if target.exists() {
        return Ok(());
    }

    let legacy_in_app_data_root = resolve_path_in_app_data(app, STORE_FILE_NAME)?;
    let misplaced_in_app_data_logs =
        resolve_path_in_app_data(app, &format!("logs/{STORE_FILE_NAME}"))?;
    let misplaced_in_root_logs = app_paths::app_log_dir(app)?.join(STORE_FILE_NAME);

    let candidates = [
        legacy_in_app_data_root,
        misplaced_in_app_data_logs,
        misplaced_in_root_logs,
    ];

    let source = candidates
        .iter()
        .find(|path| path.exists() && path.is_file())
        .cloned();
    let Some(source) = source else {
        return Ok(());
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Invariant(format!("创建数据目录失败：{e}")))?;
    }

    // 优先使用 rename（同盘移动更快）；失败时回退到 copy + remove。
    match std::fs::rename(&source, &target) {
        Ok(()) => {
            tracing::info!(
                target: "storage",
                "已迁移 store 文件：{} -> {}",
                source.to_string_lossy(),
                target.to_string_lossy()
            );
            Ok(())
        }
        Err(rename_err) => {
            std::fs::copy(&source, &target).map_err(|e| {
                AppError::Invariant(format!(
                    "迁移 store 失败（rename={rename_err}；copy 也失败）：{e}"
                ))
            })?;
            let _ = std::fs::remove_file(&source);
            tracing::info!(
                target: "storage",
                "已迁移 store 文件（copy fallback）：{} -> {}",
                source.to_string_lossy(),
                target.to_string_lossy()
            );
            Ok(())
        }
    }
}

/// 从 store 中加载 `AppData`；若为空则写入默认值并返回。
#[cfg(not(test))]
fn load_or_init_app_data(store: &tauri_plugin_store::Store<tauri::Wry>) -> AppResult<AppData> {
    if let Some(value) = store.get(STORE_KEY) {
        let mut data: AppData = serde_json::from_value(value)?;
        tracing::info!(target: "storage", "已从 store 加载 AppData");
        let mut changed = false;
        if data.migrate_v2() {
            changed = true;
        }
        if data.migrate_v4() {
            changed = true;
        }
        if changed {
            store.set(STORE_KEY, serde_json::to_value(&data)?);
            store.save()?;
            tracing::info!(target: "storage", "已完成 AppData 迁移并写回 store");
        }
        return Ok(data);
    }

    let data = AppData::default();
    store.set(STORE_KEY, serde_json::to_value(&data)?);
    store.save()?;
    tracing::info!(target: "storage", "首次启动：已写入默认 AppData");
    Ok(data)
}

/// 将窗口关闭行为改为“隐藏到托盘”（满足 PRD 的“最小化到托盘”）。
#[cfg(not(test))]
fn setup_window_close_to_tray(app: &mut tauri::App) -> AppResult<()> {
    use tauri::Manager as _;
    use tauri::WindowEvent;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| errors::AppError::Invariant("主窗口 `main` 不存在".to_string()))?;

    let window_for_event = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window_for_event.hide();
        }
    });

    Ok(())
}
