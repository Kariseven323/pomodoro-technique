//! 前端可调用的 Tauri 命令集合（invoke handler）。

use serde::{Deserialize, Serialize};
use tauri::Manager as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::app_data::{BlacklistItem, Settings};
use crate::errors::{AppError, AppResult};
use crate::processes::{self, ProcessInfo};
use crate::state::AppState;
use crate::timer::{self, TimerSnapshot, TodayStats};

/// 将内部 `AppResult<T>` 转为可被 Tauri IPC 接受的 `Result<T, String>`。
fn to_ipc_result<T>(result: AppResult<T>) -> Result<T, String> {
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

/// 前端初始化所需的完整快照（持久化数据 + 计时器状态）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    /// 持久化数据（settings/blacklist/tags/history）。
    pub data: crate::app_data::AppData,
    /// 计时器状态快照。
    pub timer: TimerSnapshot,
}

/// 应用数据（store）文件路径信息（用于设置页展示与“打开文件夹”入口）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePaths {
    /// Store 所在目录路径（可用于打开文件夹）。
    pub store_dir_path: String,
}

/// 获取应用完整快照（用于前端首屏渲染与恢复）。
#[tauri::command]
pub fn get_app_snapshot(state: tauri::State<'_, AppState>) -> Result<AppSnapshot, String> {
    to_ipc_result(Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    }))
}

/// 获取应用持久化 store 的真实存储路径（目录 + 文件路径）。
#[tauri::command]
pub fn get_store_paths(app: tauri::AppHandle) -> Result<StorePaths, String> {
    to_ipc_result(get_store_paths_impl(&app))
}

/// 打开 store 所在目录（文件管理器）。
#[tauri::command]
pub fn open_store_dir(app: tauri::AppHandle) -> Result<(), String> {
    to_ipc_result(open_store_dir_impl(&app))
}

/// 获取 store 路径的内部实现（便于统一错误处理）。
fn get_store_paths_impl(app: &tauri::AppHandle) -> AppResult<StorePaths> {
    let store_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Invariant("无法解析应用数据目录（app_data_dir）".to_string()))?;

    Ok(StorePaths {
        store_dir_path: store_dir.to_string_lossy().to_string(),
    })
}

/// 打开 store 目录的内部实现：确保目录存在后再请求系统打开。
fn open_store_dir_impl(app: &tauri::AppHandle) -> AppResult<()> {
    let store_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Invariant("无法解析应用数据目录（app_data_dir）".to_string()))?;

    std::fs::create_dir_all(&store_dir)
        .map_err(|e| AppError::Invariant(format!("创建应用数据目录失败：{e}")))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&store_dir)
            .spawn()
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        app.opener()
            .open_path(store_dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
    }

    Ok(())
}

/// 更新设置（带范围校验），并在必要时重置当前阶段的剩余时间。
#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<AppSnapshot, String> {
    to_ipc_result(update_settings_impl(&state, settings))
}

/// 更新设置的内部实现（便于统一错误处理与托盘复用）。
fn update_settings_impl(state: &AppState, settings: Settings) -> AppResult<AppSnapshot> {
    timer::validate_settings(&settings)?;

    state.update_data_and_timer(
        |data, timer_runtime| {
            data.settings = settings.clone();

            // 若当前未运行，则根据阶段同步剩余时间，以保证 UI 与设置一致。
            if !timer_runtime.is_running {
                match timer_runtime.phase {
                    crate::timer::Phase::Work => {
                        timer_runtime.remaining_seconds = settings.pomodoro as u64 * 60;
                    }
                    crate::timer::Phase::ShortBreak => {
                        timer_runtime.remaining_seconds = settings.short_break as u64 * 60;
                    }
                    crate::timer::Phase::LongBreak => {
                        timer_runtime.remaining_seconds = settings.long_break as u64 * 60;
                    }
                }
            }
            Ok(())
        },
        true,
    )?;

    let _ = crate::tray::refresh_tray(&state);
    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 设置当前番茄的任务标签；若为新标签则追加到 tags 并持久化。
#[tauri::command]
pub fn set_current_tag(
    state: tauri::State<'_, AppState>,
    tag: String,
) -> Result<AppSnapshot, String> {
    to_ipc_result(set_current_tag_impl(&state, tag))
}

/// 设置当前标签的内部实现（便于统一错误处理）。
fn set_current_tag_impl(state: &AppState, tag: String) -> AppResult<AppSnapshot> {
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }

    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.set_current_tag(tag.clone());
            if !data.tags.iter().any(|t| t == &tag) {
                data.tags.push(tag);
            }
            Ok(())
        },
        true,
    )?;

    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 新增一个标签到历史标签列表（持久化）。
#[tauri::command]
pub fn add_tag(state: tauri::State<'_, AppState>, tag: String) -> Result<Vec<String>, String> {
    to_ipc_result(add_tag_impl(&state, tag))
}

/// 新增标签的内部实现（便于统一错误处理）。
fn add_tag_impl(state: &AppState, tag: String) -> AppResult<Vec<String>> {
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }

    state.update_data(|data| {
        if !data.tags.iter().any(|t| t == &tag) {
            data.tags.push(tag);
        }
        Ok(())
    })?;

    Ok(state.data_snapshot().tags)
}

/// 设置黑名单（专注期内仅允许“新增”，不允许移除）。
#[tauri::command]
pub fn set_blacklist(
    state: tauri::State<'_, AppState>,
    blacklist: Vec<BlacklistItem>,
) -> Result<Vec<BlacklistItem>, String> {
    to_ipc_result(set_blacklist_impl(&state, blacklist))
}

/// 设置黑名单的内部实现（便于统一错误处理）。
fn set_blacklist_impl(state: &AppState, blacklist: Vec<BlacklistItem>) -> AppResult<Vec<BlacklistItem>> {
    validate_blacklist_items(&blacklist)?;

    let (added_names, should_kill_added) = state.update_data_and_timer(
        |data, timer_runtime| {
            let locked = timer_runtime.blacklist_locked();

            if locked {
                let old_names: std::collections::BTreeSet<String> = data
                    .blacklist
                    .iter()
                    .map(|b| normalize_name(&b.name))
                    .collect();
                let new_names: std::collections::BTreeSet<String> =
                    blacklist.iter().map(|b| normalize_name(&b.name)).collect();

                if !old_names.is_subset(&new_names) {
                    return Err(AppError::BlacklistLocked);
                }
            }

            let old_names: std::collections::BTreeSet<String> = data
                .blacklist
                .iter()
                .map(|b| normalize_name(&b.name))
                .collect();

            let added: Vec<String> = blacklist
                .iter()
                .filter(|b| !old_names.contains(&normalize_name(&b.name)))
                .map(|b| b.name.clone())
                .collect();

            data.blacklist = blacklist.clone();

            // PRD：番茄周期内可动态添加并立即终止。
            let should_kill = locked && !added.is_empty();

            Ok((added, should_kill))
        },
        true,
    )?;

    if should_kill_added {
        kill_names_and_emit(&state, &added_names);
    }

    Ok(state.data_snapshot().blacklist)
}

/// 获取当前运行的进程列表（进程名 + 图标）。
#[tauri::command]
pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    to_ipc_result(processes::list_processes())
}

/// 开始计时（若处于工作阶段首次开始，则终止黑名单进程）。
#[tauri::command]
pub fn timer_start(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_start_impl(&state))
}

/// 托盘复用：开始计时的内部实现（不暴露给前端）。
pub fn timer_start_inner(state: &AppState) -> AppResult<()> {
    let (names_to_kill, should_kill) = state.update_data_and_timer(
        |data, timer_runtime| {
            let should_kill = timer_runtime.phase == crate::timer::Phase::Work
                && !timer_runtime.blacklist_locked()
                && !timer_runtime.is_running;
            let names: Vec<String> = data.blacklist.iter().map(|b| b.name.clone()).collect();
            timer_runtime.start();
            Ok((names, should_kill))
        },
        false,
    )?;

    if should_kill {
        kill_names_and_emit(state, &names_to_kill);
    }

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 开始计时的 IPC 入口实现：先执行内部逻辑，再返回快照。
fn timer_start_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_start_inner(state)?;
    Ok(state.timer_snapshot())
}

/// 暂停计时。
#[tauri::command]
pub fn timer_pause(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_pause_impl(&state))
}

/// 托盘复用：暂停计时的内部实现（不暴露给前端）。
pub fn timer_pause_inner(state: &AppState) -> AppResult<()> {
    state.update_timer(|timer_runtime, _data| {
        timer_runtime.pause();
        Ok(())
    })?;

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(())
}

/// 暂停计时的 IPC 入口实现：先执行内部逻辑，再返回快照。
fn timer_pause_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    timer_pause_inner(state)?;
    Ok(state.timer_snapshot())
}

/// 重置计时（回到工作阶段初始状态）。
#[tauri::command]
pub fn timer_reset(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_reset_impl(&state))
}

/// 重置计时的内部实现（便于统一错误处理）。
fn timer_reset_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.reset(&data.settings);
            Ok(())
        },
        false,
    )?;

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(state.timer_snapshot())
}

/// 跳过当前阶段（不写入历史）。
#[tauri::command]
pub fn timer_skip(state: tauri::State<'_, AppState>) -> Result<TimerSnapshot, String> {
    to_ipc_result(timer_skip_impl(&state))
}

/// 跳过阶段的内部实现（便于统一错误处理）。
fn timer_skip_impl(state: &AppState) -> AppResult<TimerSnapshot> {
    state.update_data_and_timer(
        |data, timer_runtime| {
            let completed_today = TodayStats::from_app_data(data).total;
            timer_runtime.skip(&data.settings, completed_today);
            Ok(())
        },
        false,
    )?;

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();
    Ok(state.timer_snapshot())
}

/// Windows：以管理员身份重启（用于终止需要提权的进程）。
#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    to_ipc_result(restart_as_admin_impl())
}

/// 提权重启的内部实现（便于统一错误处理）。
fn restart_as_admin_impl() -> AppResult<()> {
    #[cfg(windows)]
    {
        restart_as_admin_windows()
    }
    #[cfg(not(windows))]
    {
        Err(AppError::UnsupportedPlatform(
            "仅 Windows 支持“以管理员身份重启”".to_string(),
        ))
    }
}

/// Windows：通过 `ShellExecuteW` 使用 `runas` 重新启动自身并退出。
#[cfg(windows)]
fn restart_as_admin_windows() -> AppResult<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe()
        .map_err(|e| AppError::Invariant(format!("获取当前可执行文件失败：{e}")))?;
    let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let exe_w = to_wide_null(exe.to_string_lossy().as_ref());
    let verb_w = to_wide_null("runas");
    let args_w = to_wide_null(&args);

    unsafe {
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(verb_w.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(args_w.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        let code = result.0 as isize;
        if code <= 32 {
            return Err(AppError::Invariant(format!(
                "提权重启失败（ShellExecute 返回 {code:?}）",
            )));
        }
    }

    std::process::exit(0);
}

/// Windows：将 `&str` 转为以 `\\0` 结尾的 UTF-16。
#[cfg(windows)]
fn to_wide_null(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// 校验黑名单条目：名称不能为空、不得重复（忽略大小写）。
fn validate_blacklist_items(items: &[BlacklistItem]) -> AppResult<()> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    for it in items {
        if it.name.trim().is_empty() {
            return Err(AppError::Validation("黑名单进程名不能为空".to_string()));
        }
        if it.display_name.trim().is_empty() {
            return Err(AppError::Validation("黑名单显示名不能为空".to_string()));
        }
        let key = normalize_name(&it.name);
        if !seen.insert(key) {
            return Err(AppError::Validation("黑名单存在重复进程名".to_string()));
        }
    }
    Ok(())
}

/// 规范化进程名用于比较（Windows 下大小写不敏感）。
fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// 批量终止若干进程名，并将结果通过事件推送到前端。
fn kill_names_and_emit(state: &AppState, names: &[String]) {
    if names.is_empty() {
        return;
    }

    let mut all_items = Vec::new();
    let mut requires_admin = false;

    for name in names {
        if let Ok(summary) = processes::kill_by_name(name) {
            requires_admin |= summary.requires_admin;
            all_items.extend(summary.items);
        }
    }

    let payload = processes::KillSummary {
        items: all_items,
        requires_admin,
    };

    let _ = state.emit_kill_result(payload);
}
