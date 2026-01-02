//! 模板相关 IPC 命令：将前端调用转发到可测试的命令逻辑实现。

use crate::app_data::{BlacklistItem, BlacklistTemplate};
use crate::commands::common::to_ipc_result;
use crate::commands::templates::{apply_template_impl, delete_template_impl, get_templates_impl, save_template_impl};
use crate::errors::AppResult;
use crate::state::AppState;

/// 获取模板列表与当前激活模板状态。
#[tauri::command]
pub fn get_templates(state: tauri::State<'_, AppState>) -> Result<Vec<BlacklistTemplate>, String> {
    to_ipc_result(get_templates_ipc_impl(&*state))
}

/// 保存模板：新增或更新（自定义模板）。
#[tauri::command]
pub fn save_template(
    state: tauri::State<'_, AppState>,
    template: BlacklistTemplate,
) -> Result<BlacklistTemplate, String> {
    to_ipc_result(save_template_ipc_impl(&*state, template))
}

/// 删除模板（禁止删除内置模板）。
#[tauri::command]
pub fn delete_template(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    to_ipc_result(delete_template_ipc_impl(&*state, id))
}

/// 应用模板：切换黑名单到模板内容，并同步激活模板 id。
#[tauri::command]
pub fn apply_template(state: tauri::State<'_, AppState>, id: String) -> Result<Vec<BlacklistItem>, String> {
    to_ipc_result(apply_template_ipc_impl(&*state, id))
}

/// IPC 内部实现：复用 `commands::templates` 的可测试实现。
fn get_templates_ipc_impl(state: &AppState) -> AppResult<Vec<BlacklistTemplate>> {
    get_templates_impl(state)
}

/// IPC 内部实现：复用 `commands::templates` 的可测试实现。
fn save_template_ipc_impl(state: &AppState, template: BlacklistTemplate) -> AppResult<BlacklistTemplate> {
    save_template_impl(state, template)
}

/// IPC 内部实现：复用 `commands::templates` 的可测试实现。
fn delete_template_ipc_impl(state: &AppState, id: String) -> AppResult<bool> {
    delete_template_impl(state, id)
}

/// IPC 内部实现：复用 `commands::templates` 的可测试实现。
fn apply_template_ipc_impl(state: &AppState, id: String) -> AppResult<Vec<BlacklistItem>> {
    apply_template_impl(state, id)
}
