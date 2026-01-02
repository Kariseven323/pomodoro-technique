//! 前端可调用的 Tauri 命令集合（invoke handler）。

use serde::{Deserialize, Serialize};
use tauri::LogicalPosition;
use tauri::LogicalSize;
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
#[cfg(not(windows))]
use tauri_plugin_opener::OpenerExt as _;

use crate::analysis::FocusAnalysis;
use crate::app_data::{
    BlacklistItem, BlacklistTemplate, DateRange, HistoryDay, HistoryRecord, Phase, Settings,
};
use crate::app_paths;
use crate::errors::{AppError, AppResult};
use crate::logging;
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

/// 应用数据根目录路径信息（用于设置页展示与“打开文件夹”入口）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePaths {
    /// 数据根目录路径（统一入口，可用于打开文件夹）。
    pub store_dir_path: String,
}

/// 导出格式（CSV/JSON）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    /// CSV（逗号分隔）。
    Csv,
    /// JSON（结构化）。
    Json,
}

/// 导出字段（用于“自选导出字段”）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportField {
    /// 日期（YYYY-MM-DD）。
    Date,
    /// 开始时间（HH:mm）。
    StartTime,
    /// 结束时间（HH:mm）。
    EndTime,
    /// 时长（分钟）。
    Duration,
    /// 标签。
    Tag,
    /// 阶段类型（work/shortBreak/longBreak）。
    Phase,
    /// 备注（PRD v2 新增，可选导出）。
    Remark,
}

/// 导出请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    /// 导出范围。
    pub range: DateRange,
    /// 导出格式。
    pub format: ExportFormat,
    /// 导出字段（为空则导出默认字段集）。
    #[serde(default)]
    pub fields: Vec<ExportField>,
}

/// 向前端广播“调试历史数据变更”的事件名（用于自动刷新历史页面）。
pub const EVENT_HISTORY_DEV_CHANGED: &str = "pomodoro://history_dev_changed";

/// 获取应用完整快照（用于前端首屏渲染与恢复）。
#[tauri::command]
pub fn get_app_snapshot(state: tauri::State<'_, AppState>) -> Result<AppSnapshot, String> {
    to_ipc_result(Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    }))
}

/// 获取应用数据根目录路径（统一入口，便于用户一处查看与备份）。
#[tauri::command]
pub fn get_store_paths(app: tauri::AppHandle) -> Result<StorePaths, String> {
    to_ipc_result(get_store_paths_impl(&app))
}

/// 打开应用数据根目录（文件管理器，统一入口）。
#[tauri::command]
pub fn open_store_dir(app: tauri::AppHandle) -> Result<(), String> {
    to_ipc_result(open_store_dir_impl(&app))
}

/// 获取应用数据根目录路径的内部实现（统一入口）。
fn get_store_paths_impl(app: &tauri::AppHandle) -> AppResult<StorePaths> {
    let store_dir = app_paths::app_root_dir(app)?;

    Ok(StorePaths {
        store_dir_path: store_dir.to_string_lossy().to_string(),
    })
}

/// 打开应用数据根目录的内部实现：确保目录存在后再请求系统打开（统一入口）。
fn open_store_dir_impl(app: &tauri::AppHandle) -> AppResult<()> {
    let store_dir = app_paths::app_root_dir(app)?;

    std::fs::create_dir_all(&store_dir)
        .map_err(|e| AppError::Invariant(format!("创建根目录失败：{e}")))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&store_dir)
            .spawn()
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        app.opener()
            .open_path(store_dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| AppError::Invariant(format!("打开文件夹失败：{e}")))?;
        Ok(())
    }
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

    tracing::info!(
        target: "storage",
        "更新设置：pomodoro={} shortBreak={} longBreak={} longBreakInterval={} dailyGoal={} weeklyGoal={} alwaysOnTop={}",
        settings.pomodoro,
        settings.short_break,
        settings.long_break,
        settings.long_break_interval,
        settings.daily_goal,
        settings.weekly_goal,
        settings.always_on_top
    );

    state.update_data_and_timer(
        |data, timer_runtime| {
            data.settings = settings.clone();

            // 若当前未运行，则根据阶段同步剩余时间，以保证 UI 与设置一致。
            if !timer_runtime.is_running {
                match timer_runtime.phase {
                    Phase::Work => {
                        timer_runtime.remaining_seconds = settings.pomodoro as u64 * 60;
                    }
                    Phase::ShortBreak => {
                        timer_runtime.remaining_seconds = settings.short_break as u64 * 60;
                    }
                    Phase::LongBreak => {
                        timer_runtime.remaining_seconds = settings.long_break as u64 * 60;
                    }
                }
            }
            Ok(())
        },
        true,
    )?;

    let _ = crate::tray::refresh_tray(state);
    let _ = state.emit_timer_snapshot();

    Ok(AppSnapshot {
        data: state.data_snapshot(),
        timer: state.timer_snapshot(),
    })
}

/// 设置每日/每周目标（0 表示不设目标），并持久化到 settings。
#[tauri::command]
pub fn set_goals(
    state: tauri::State<'_, AppState>,
    daily: u32,
    weekly: u32,
) -> Result<Settings, String> {
    to_ipc_result(set_goals_impl(&state, daily, weekly))
}

/// 设置目标的内部实现（便于统一错误处理）。
fn set_goals_impl(state: &AppState, daily: u32, weekly: u32) -> AppResult<Settings> {
    let mut next = state.data_snapshot().settings;
    next.daily_goal = daily;
    next.weekly_goal = weekly;
    timer::validate_settings(&next)?;

    state.update_data(|data| {
        data.settings.daily_goal = daily;
        data.settings.weekly_goal = weekly;
        Ok(())
    })?;

    tracing::info!(target: "timer", "设置目标：dailyGoal={} weeklyGoal={}", daily, weekly);
    let _ = state.emit_timer_snapshot();

    Ok(state.data_snapshot().settings)
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
fn set_blacklist_impl(
    state: &AppState,
    blacklist: Vec<BlacklistItem>,
) -> AppResult<Vec<BlacklistItem>> {
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
        tracing::info!(target: "blacklist", "专注期新增黑名单条目，立即尝试终止：{:?}", added_names);
        kill_names_and_emit(state, &added_names);
    }

    Ok(state.data_snapshot().blacklist)
}

/// 获取指定日期范围内的历史记录（按日分组）。
#[tauri::command]
pub fn get_history(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<HistoryDay>, String> {
    to_ipc_result(get_history_impl(&state, &range))
}

/// 获取历史的内部实现：校验日期范围后按 `YYYY-MM-DD` 字符串过滤（闭区间）。
fn get_history_impl(state: &AppState, range: &DateRange) -> AppResult<Vec<HistoryDay>> {
    validate_date_range(range)?;

    let data = state.data_snapshot();
    let mut out: Vec<HistoryDay> = history_for_ui(&data)
        .iter()
        .filter(|d| d.date >= range.from && d.date <= range.to)
        .cloned()
        .collect();

    // 让 UI 的“默认本周”更自然：按日期倒序展示。
    out.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(out)
}

/// 设置某条历史记录的备注（用于“完成后填写”与“历史中编辑”）。
#[tauri::command]
pub fn set_history_remark(
    state: tauri::State<'_, AppState>,
    date: String,
    record_index: usize,
    remark: String,
) -> Result<HistoryRecord, String> {
    to_ipc_result(set_history_remark_impl(&state, date, record_index, remark))
}

/// 设置备注的内部实现：按日期 + 索引定位并持久化。
fn set_history_remark_impl(
    state: &AppState,
    date: String,
    record_index: usize,
    remark: String,
) -> AppResult<HistoryRecord> {
    let date = date.trim().to_string();
    validate_ymd(&date)?;

    let remark = remark.trim().to_string();
    state.update_data(|data| {
        let list = history_for_ui_mut(data);
        let Some(day) = list.iter_mut().find(|d| d.date == date) else {
            return Err(AppError::Validation("找不到指定日期的历史记录".to_string()));
        };
        if record_index >= day.records.len() {
            return Err(AppError::Validation("历史记录索引超出范围".to_string()));
        }
        day.records[record_index].remark = remark.clone();
        Ok(())
    })?;

    tracing::info!(target: "storage", "更新历史备注：date={} index={}", date, record_index);
    let data = state.data_snapshot();
    let day = history_for_ui(&data)
        .iter()
        .find(|d| d.date == date)
        .ok_or_else(|| AppError::Invariant("写入后读取历史失败".to_string()))?;
    Ok(day.records[record_index].clone())
}

/// 获取指定范围的专注分析数据（用于“专注时段分析”图表/摘要）。
#[tauri::command]
pub fn get_focus_analysis(
    state: tauri::State<'_, AppState>,
    range: DateRange,
) -> Result<FocusAnalysis, String> {
    to_ipc_result(get_focus_analysis_impl(&state, &range))
}

/// 获取专注分析的内部实现。
fn get_focus_analysis_impl(state: &AppState, range: &DateRange) -> AppResult<FocusAnalysis> {
    validate_date_range(range)?;
    let data = state.data_snapshot();
    crate::analysis::get_focus_analysis(history_for_ui(&data), range)
}

/// 获取全部黑名单模板（包含内置与自定义）。
#[tauri::command]
pub fn get_templates(state: tauri::State<'_, AppState>) -> Result<Vec<BlacklistTemplate>, String> {
    to_ipc_result(get_templates_impl(&state))
}

/// 获取模板的内部实现。
fn get_templates_impl(state: &AppState) -> AppResult<Vec<BlacklistTemplate>> {
    Ok(state.data_snapshot().blacklist_templates)
}

/// 保存模板：新增或更新自定义模板（内置模板不可覆盖）。
#[tauri::command]
pub fn save_template(
    state: tauri::State<'_, AppState>,
    template: BlacklistTemplate,
) -> Result<BlacklistTemplate, String> {
    to_ipc_result(save_template_impl(&state, template))
}

/// 保存模板的内部实现：校验字段并持久化。
fn save_template_impl(
    state: &AppState,
    mut template: BlacklistTemplate,
) -> AppResult<BlacklistTemplate> {
    template.name = template.name.trim().to_string();
    if template.name.is_empty() {
        return Err(AppError::Validation("模板名称不能为空".to_string()));
    }
    if template.builtin {
        return Err(AppError::Validation("内置模板不可保存覆盖".to_string()));
    }

    if template.id.trim().is_empty() {
        let ts = chrono::Utc::now().timestamp_millis();
        template.id = format!("custom-{ts}");
    }

    validate_blacklist_items(&template.processes)?;

    state.update_data(|data| {
        if let Some(existing) = data
            .blacklist_templates
            .iter()
            .find(|t| t.id == template.id)
        {
            if existing.builtin {
                return Err(AppError::Validation("内置模板不可覆盖".to_string()));
            }
        }

        let mut next: Vec<BlacklistTemplate> = Vec::new();
        let mut replaced = false;
        for t in &data.blacklist_templates {
            if t.id == template.id {
                next.push(template.clone());
                replaced = true;
            } else {
                next.push(t.clone());
            }
        }
        if !replaced {
            next.push(template.clone());
        }
        data.blacklist_templates = next;
        Ok(())
    })?;

    tracing::info!(target: "storage", "保存模板：id={} name={}", template.id, template.name);
    Ok(template)
}

/// 删除自定义模板（内置模板不可删除）。
#[tauri::command]
pub fn delete_template(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    to_ipc_result(delete_template_impl(&state, id))
}

/// 删除模板的内部实现。
fn delete_template_impl(state: &AppState, id: String) -> AppResult<bool> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err(AppError::Validation("模板 id 不能为空".to_string()));
    }

    let mut deleted = false;
    state.update_data(|data| {
        let Some(existing) = data.blacklist_templates.iter().find(|t| t.id == id) else {
            return Ok(());
        };
        if existing.builtin {
            return Err(AppError::Validation("内置模板不可删除".to_string()));
        }

        data.blacklist_templates = data
            .blacklist_templates
            .iter()
            .filter(|t| t.id != id)
            .cloned()
            .collect();

        data.active_template_ids.retain(|x| x != &id);
        data.active_template_id = data.active_template_ids.first().cloned();
        deleted = true;
        Ok(())
    })?;

    if deleted {
        tracing::info!(target: "storage", "删除模板：id={}", id);
    }

    Ok(deleted)
}

/// 应用/切换模板：支持同时启用多套模板；返回应用后的黑名单。
#[tauri::command]
pub fn apply_template(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<BlacklistItem>, String> {
    to_ipc_result(apply_template_impl(&state, id))
}

/// 应用模板的内部实现：在专注期锁定时禁止切换模板。
fn apply_template_impl(state: &AppState, id: String) -> AppResult<Vec<BlacklistItem>> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err(AppError::Validation("模板 id 不能为空".to_string()));
    }

    let locked = state.timer_snapshot().blacklist_locked;
    if locked {
        return Err(AppError::BlacklistLocked);
    }

    state.update_data(|data| {
        let exists = data.blacklist_templates.iter().any(|t| t.id == id);
        if !exists {
            return Err(AppError::Validation("模板不存在".to_string()));
        }

        if data.active_template_ids.iter().any(|x| x == &id) {
            data.active_template_ids.retain(|x| x != &id);
        } else {
            data.active_template_ids.push(id.clone());
        }

        data.active_template_ids.sort();
        data.active_template_id = data.active_template_ids.first().cloned();
        data.blacklist = compute_blacklist_from_active_templates(data);
        Ok(())
    })?;

    let out = state.data_snapshot().blacklist;
    tracing::info!(target: "blacklist", "应用模板后黑名单条目数={}", out.len());
    Ok(out)
}

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

/// 导出历史记录：弹出保存对话框并写入 CSV/JSON，返回保存的文件路径。
#[tauri::command]
pub async fn export_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<String, String> {
    to_ipc_result(export_history_impl(&app, &state, request))
}

/// 导出历史的内部实现：校验范围、弹出保存对话框、写入文件。
fn export_history_impl(
    app: &tauri::AppHandle,
    state: &AppState,
    request: ExportRequest,
) -> AppResult<String> {
    validate_date_range(&request.range)?;

    let mut fields = request.fields;
    if fields.is_empty() {
        fields = vec![
            ExportField::Date,
            ExportField::StartTime,
            ExportField::EndTime,
            ExportField::Duration,
            ExportField::Tag,
            ExportField::Phase,
        ];
    }

    let ext = match request.format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };

    let default_name = format!(
        "pomodoro-history-{}-{}.{}",
        request.range.from, request.range.to, ext
    );

    // 后端弹出系统保存对话框（PRD v2）。
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file()
    else {
        return Err(AppError::Validation("已取消导出".to_string()));
    };

    let path = path
        .into_path()
        .map_err(|_| AppError::Invariant("导出路径解析失败".to_string()))?;

    let days = get_history_impl(state, &request.range)?;
    let export_rows = flatten_days_to_rows(&days);

    match request.format {
        ExportFormat::Csv => export_csv(&path, &fields, &export_rows)?,
        ExportFormat::Json => export_json(&path, &request.range, &export_rows)?,
    }

    tracing::info!(target: "storage", "导出历史成功：path={}", path.to_string_lossy());
    Ok(path.to_string_lossy().to_string())
}

/// 打开日志目录（文件管理器）。
#[tauri::command]
pub fn open_log_dir(app: tauri::AppHandle) -> Result<bool, String> {
    to_ipc_result(open_log_dir_impl(&app))
}

/// 前端日志桥接：将前端诊断信息写入后端文件日志（用于定位 WebView/布局问题）。
#[tauri::command]
pub fn frontend_log(level: String, message: String) -> Result<bool, String> {
    to_ipc_result(frontend_log_impl(&level, &message))
}

/// 前端日志桥接的内部实现：按 level 写入 tracing。
fn frontend_log_impl(level: &str, message: &str) -> AppResult<bool> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation(
            "仅开发环境可使用前端诊断日志（frontend_log）".to_string(),
        ));
    }

    let lvl = level.trim().to_lowercase();
    match lvl.as_str() {
        "debug" => tracing::debug!(target: "frontend", "{message}"),
        "warn" | "warning" => tracing::warn!(target: "frontend", "{message}"),
        "error" => tracing::error!(target: "frontend", "{message}"),
        _ => tracing::info!(target: "frontend", "{message}"),
    }
    Ok(true)
}

/// 打开日志目录的内部实现。
fn open_log_dir_impl(app: &tauri::AppHandle) -> AppResult<bool> {
    let dir = logging::log_dir(app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Invariant(format!("创建日志目录失败：{e}")))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| AppError::Invariant(format!("打开日志目录失败：{e}")))?;
        return Ok(true);
    }

    #[cfg(not(windows))]
    {
        app.opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| AppError::Invariant(format!("打开日志目录失败：{e}")))?;
        Ok(true)
    }
}

/// 开发者命令：一键生成测试历史数据并写入 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_generate_history(state: tauri::State<'_, AppState>, days: u32) -> Result<u32, String> {
    to_ipc_result(debug_generate_history_impl(&state, days))
}

/// 开发者命令：清空 `history_dev`（仅开发环境可用）。
#[tauri::command]
pub fn debug_clear_history(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    to_ipc_result(debug_clear_history_impl(&state))
}

/// 生成调试历史数据的内部实现：校验参数、写入 store、并通知前端刷新。
fn debug_generate_history_impl(state: &AppState, days: u32) -> AppResult<u32> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation("仅开发环境可使用调试模式".to_string()));
    }
    if !(1..=365).contains(&days) {
        return Err(AppError::Validation("天数需在 1-365".to_string()));
    }

    let mut generated = 0u32;

    state.update_data(|data| {
        let settings = data.settings.clone();
        let tags = if data.tags.is_empty() {
            vec!["工作".to_string()]
        } else {
            data.tags.clone()
        };

        let (history, count) = generate_history_dev(days, &settings, &tags);
        data.history_dev = history;
        generated = count;
        Ok(())
    })?;

    tracing::info!(target: "storage", "生成测试历史数据：days={} records={}", days, generated);
    let _ = state.emit_simple_event(EVENT_HISTORY_DEV_CHANGED);
    Ok(generated)
}

/// 清除调试历史数据的内部实现：清空 `history_dev` 并通知前端刷新。
fn debug_clear_history_impl(state: &AppState) -> AppResult<bool> {
    if !cfg!(debug_assertions) {
        return Err(AppError::Validation("仅开发环境可使用调试模式".to_string()));
    }

    state.update_data(|data| {
        data.history_dev = Vec::new();
        Ok(())
    })?;

    tracing::info!(target: "storage", "已清除测试历史数据（history_dev）");
    let _ = state.emit_simple_event(EVENT_HISTORY_DEV_CHANGED);
    Ok(true)
}

/// 退出应用（用于迷你模式右键菜单）。
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) -> Result<bool, String> {
    to_ipc_result(exit_app_impl(&app))
}

/// 退出应用的内部实现：请求 Tauri 退出。
fn exit_app_impl(app: &tauri::AppHandle) -> AppResult<bool> {
    tracing::info!(target: "system", "请求退出应用");
    app.exit(0);
    Ok(true)
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
            let should_kill = timer_runtime.phase == Phase::Work
                && !timer_runtime.blacklist_locked()
                && !timer_runtime.is_running;
            let names: Vec<String> = data.blacklist.iter().map(|b| b.name.clone()).collect();
            timer_runtime.start(&data.settings);
            Ok((names, should_kill))
        },
        false,
    )?;

    tracing::info!(
        target: "timer",
        "开始计时：phase={:?} tag={} remaining={}s",
        state.timer_snapshot().phase,
        state.timer_snapshot().current_tag,
        state.timer_snapshot().remaining_seconds
    );

    if should_kill {
        tracing::info!(target: "blacklist", "工作阶段首次开始，尝试终止黑名单进程：{:?}", names_to_kill);
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

    tracing::info!(
        target: "timer",
        "暂停计时：phase={:?} remaining={}s",
        state.timer_snapshot().phase,
        state.timer_snapshot().remaining_seconds
    );

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

    tracing::info!(target: "timer", "重置计时器：回到工作阶段");
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

    tracing::info!(target: "timer", "跳过阶段：phase={:?}", state.timer_snapshot().phase);
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
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
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

/// 校验日期字符串是否符合 `YYYY-MM-DD`。
fn validate_ymd(date: &str) -> AppResult<()> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("日期格式必须为 YYYY-MM-DD".to_string()))?;
    Ok(())
}

/// 校验日期范围：格式正确且 `from <= to`。
fn validate_date_range(range: &DateRange) -> AppResult<()> {
    validate_ymd(range.from.trim())?;
    validate_ymd(range.to.trim())?;
    if range.from.trim() > range.to.trim() {
        return Err(AppError::Validation(
            "日期范围不合法：from 不能晚于 to".to_string(),
        ));
    }
    Ok(())
}

/// 选择供“历史页面/导出/分析”使用的历史数据源（开发环境：优先 `history_dev`）。
fn history_for_ui(data: &crate::app_data::AppData) -> &Vec<HistoryDay> {
    if cfg!(debug_assertions) && !data.history_dev.is_empty() {
        &data.history_dev
    } else {
        &data.history
    }
}

/// 选择供“历史备注编辑”使用的可变历史数据源（开发环境：优先 `history_dev`）。
fn history_for_ui_mut(data: &mut crate::app_data::AppData) -> &mut Vec<HistoryDay> {
    if cfg!(debug_assertions) && !data.history_dev.is_empty() {
        &mut data.history_dev
    } else {
        &mut data.history
    }
}

/// 生成 `history_dev`：返回按日分组的历史与生成的记录总数。
fn generate_history_dev(days: u32, settings: &Settings, tags: &[String]) -> (Vec<HistoryDay>, u32) {
    use chrono::{Datelike as _, Duration as ChronoDuration, Local, NaiveDate, Weekday};
    use rand::Rng as _;

    let mut rng = rand::thread_rng();
    let today: NaiveDate = Local::now().date_naive();
    let start = today - ChronoDuration::days((days as i64).saturating_sub(1));

    let mut out: Vec<HistoryDay> = Vec::new();
    let mut total = 0u32;

    for offset in 0..days {
        let date = start + ChronoDuration::days(offset as i64);
        let weekday = date.weekday();
        let base = rng.gen_range(4..=8);
        let daily_count = if weekday == Weekday::Sat || weekday == Weekday::Sun {
            (base / 2).max(1)
        } else {
            base
        };

        let mut records: Vec<HistoryRecord> = Vec::new();
        for _ in 0..daily_count {
            let phase_roll: u8 = rng.gen_range(0..=99);
            let phase = if phase_roll < 80 {
                Phase::Work
            } else if phase_roll < 90 {
                Phase::ShortBreak
            } else {
                Phase::LongBreak
            };

            let duration = match phase {
                Phase::Work => {
                    let base = settings.pomodoro as i32;
                    let varied = base + rng.gen_range(-5..=5);
                    varied.clamp(1, 60) as u32
                }
                Phase::ShortBreak => settings.short_break.clamp(1, 30),
                Phase::LongBreak => settings.long_break.clamp(1, 60),
            };

            let (start_time, end_time) = random_time_in_windows(&mut rng, duration);
            let tag = pick_random_tag(&mut rng, tags);

            records.push(HistoryRecord {
                tag,
                start_time,
                end_time: Some(end_time),
                duration,
                phase,
                remark: String::new(),
            });
        }

        records.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        total += records.len() as u32;

        out.push(HistoryDay {
            date: date.format("%Y-%m-%d").to_string(),
            records,
        });
    }

    (out, total)
}

/// 从现有标签列表中随机挑选一个标签（列表为空时回退为“工作”）。
fn pick_random_tag(rng: &mut impl rand::Rng, tags: &[String]) -> String {
    if tags.is_empty() {
        return "工作".to_string();
    }
    let idx = rng.gen_range(0..tags.len());
    tags[idx].clone()
}

/// 在规定时间窗内随机生成开始/结束时间（HH:mm），并保证结束时间不超过窗末尾。
fn random_time_in_windows(rng: &mut impl rand::Rng, duration_minutes: u32) -> (String, String) {
    // 规则：9:00-12:00, 14:00-18:00
    let windows: &[(u32, u32)] = &[(9 * 60, 12 * 60), (14 * 60, 18 * 60)];
    let (start_min, end_min) = windows[rng.gen_range(0..windows.len())];
    let latest_start = end_min.saturating_sub(duration_minutes).max(start_min);
    let start = rng.gen_range(start_min..=latest_start);
    let end = start + duration_minutes;
    (minutes_to_hhmm(start), minutes_to_hhmm(end))
}

/// 将分钟数（0-1440）格式化为 `HH:mm`。
fn minutes_to_hhmm(total_minutes: u32) -> String {
    let hh = (total_minutes / 60) % 24;
    let mm = total_minutes % 60;
    format!("{:02}:{:02}", hh, mm)
}

/// 根据当前启用模板集合计算“有效黑名单”（按进程名去重，忽略大小写）。
fn compute_blacklist_from_active_templates(data: &crate::app_data::AppData) -> Vec<BlacklistItem> {
    let active: std::collections::BTreeSet<String> =
        data.active_template_ids.iter().cloned().collect();
    let mut out: Vec<BlacklistItem> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for t in &data.blacklist_templates {
        if !active.contains(&t.id) {
            continue;
        }
        for p in &t.processes {
            let key = normalize_name(&p.name);
            if seen.insert(key) {
                out.push(p.clone());
            }
        }
    }

    out
}

/// 将按日分组的历史拉平成导出行（每条记录一行）。
fn flatten_days_to_rows(days: &[HistoryDay]) -> Vec<ExportRow> {
    let mut out = Vec::new();
    for day in days {
        for r in &day.records {
            out.push(ExportRow {
                date: day.date.clone(),
                record: r.clone(),
            });
        }
    }
    out
}

/// 单条导出行：`date + record`。
#[derive(Debug, Clone)]
struct ExportRow {
    date: String,
    record: HistoryRecord,
}

/// 将 `startTime + duration` 推导出 `endTime`（用于旧数据缺失 `end_time` 的兼容）。
fn derive_end_time_hhmm(start_time: &str, duration_minutes: u32) -> Option<String> {
    let parts: Vec<&str> = start_time.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: i32 = parts[0].parse().ok()?;
    let m: i32 = parts[1].parse().ok()?;
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) {
        return None;
    }
    let total = h * 60 + m + duration_minutes as i32;
    let hh = ((total / 60) % 24 + 24) % 24;
    let mm = ((total % 60) + 60) % 60;
    Some(format!("{:02}:{:02}", hh, mm))
}

/// 导出 CSV 文件（字段可配置）。
fn export_csv(path: &std::path::Path, fields: &[ExportField], rows: &[ExportRow]) -> AppResult<()> {
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Invariant(format!("创建导出文件失败：{e}")))?;
    let mut wtr = csv::Writer::from_writer(file);

    let header: Vec<&str> = fields
        .iter()
        .map(|f| match f {
            ExportField::Date => "date",
            ExportField::StartTime => "start_time",
            ExportField::EndTime => "end_time",
            ExportField::Duration => "duration",
            ExportField::Tag => "tag",
            ExportField::Phase => "phase",
            ExportField::Remark => "remark",
        })
        .collect();
    wtr.write_record(&header)
        .map_err(|e| AppError::Invariant(format!("写入 CSV 头失败：{e}")))?;

    for row in rows {
        let mut record: Vec<String> = Vec::new();
        for f in fields {
            let v = match f {
                ExportField::Date => row.date.clone(),
                ExportField::StartTime => row.record.start_time.clone(),
                ExportField::EndTime => row
                    .record
                    .end_time
                    .clone()
                    .or_else(|| derive_end_time_hhmm(&row.record.start_time, row.record.duration))
                    .unwrap_or_default(),
                ExportField::Duration => row.record.duration.to_string(),
                ExportField::Tag => row.record.tag.clone(),
                ExportField::Phase => match row.record.phase {
                    Phase::Work => "work".to_string(),
                    Phase::ShortBreak => "shortBreak".to_string(),
                    Phase::LongBreak => "longBreak".to_string(),
                },
                ExportField::Remark => row.record.remark.clone(),
            };
            record.push(v);
        }
        wtr.write_record(&record)
            .map_err(|e| AppError::Invariant(format!("写入 CSV 行失败：{e}")))?;
    }
    wtr.flush()
        .map_err(|e| AppError::Invariant(format!("写入 CSV 失败：{e}")))?;
    Ok(())
}

/// JSON 导出文件顶层结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExport {
    export_date: String,
    range: DateRange,
    records: Vec<JsonExportRecord>,
}

/// JSON 导出单条记录结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExportRecord {
    date: String,
    start_time: String,
    end_time: String,
    duration: u32,
    tag: String,
    phase: String,
    remark: String,
}

/// 导出 JSON 文件（字段固定为 PRD v2 示例的 superset）。
fn export_json(path: &std::path::Path, range: &DateRange, rows: &[ExportRow]) -> AppResult<()> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut records: Vec<JsonExportRecord> = Vec::new();
    for row in rows {
        let end_time = row
            .record
            .end_time
            .clone()
            .or_else(|| derive_end_time_hhmm(&row.record.start_time, row.record.duration))
            .unwrap_or_default();
        let phase = match row.record.phase {
            Phase::Work => "work",
            Phase::ShortBreak => "shortBreak",
            Phase::LongBreak => "longBreak",
        }
        .to_string();
        records.push(JsonExportRecord {
            date: row.date.clone(),
            start_time: row.record.start_time.clone(),
            end_time,
            duration: row.record.duration,
            tag: row.record.tag.clone(),
            phase,
            remark: row.record.remark.clone(),
        });
    }

    let out = JsonExport {
        export_date: today,
        range: range.clone(),
        records,
    };

    let json = serde_json::to_string_pretty(&out)?;
    std::fs::write(path, json).map_err(|e| AppError::Invariant(format!("写入 JSON 失败：{e}")))?;
    Ok(())
}

/// 批量终止若干进程名，并将结果通过事件推送到前端。
pub(crate) fn kill_names_and_emit(state: &AppState, names: &[String]) {
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
