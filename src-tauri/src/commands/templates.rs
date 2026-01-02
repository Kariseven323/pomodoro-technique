//! 黑名单模板相关命令：查询/保存/删除/应用模板。

use crate::app_data::{BlacklistItem, BlacklistTemplate};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;
use super::validation::{normalize_name, validate_blacklist_items};

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
