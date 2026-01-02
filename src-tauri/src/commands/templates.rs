//! 黑名单模板相关命令：查询/保存/删除/应用模板。

use crate::app_data::{BlacklistItem, BlacklistTemplate};
use crate::errors::{AppError, AppResult};

use super::state_like::CommandState;
use super::validation::{normalize_name, validate_blacklist_items};

/// 获取模板的内部实现。
pub(crate) fn get_templates_impl<S: CommandState>(state: &S) -> AppResult<Vec<BlacklistTemplate>> {
    Ok(state.data_snapshot().blacklist_templates)
}

/// 保存模板的内部实现：校验字段并持久化。
pub(crate) fn save_template_impl<S: CommandState>(
    state: &S,
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

/// 删除模板的内部实现。
pub(crate) fn delete_template_impl<S: CommandState>(state: &S, id: String) -> AppResult<bool> {
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

/// 应用模板的内部实现：在专注期锁定时禁止切换模板。
pub(crate) fn apply_template_impl<S: CommandState>(
    state: &S,
    id: String,
) -> AppResult<Vec<BlacklistItem>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::AppData;
    use crate::commands::state_like::CommandState;
    use crate::commands::state_like::TestState;

    /// `get_templates_impl`：应直接返回当前模板列表。
    #[test]
    fn get_templates_returns_templates() {
        let data = AppData::default();
        let state = TestState::new(data.clone());
        let out = get_templates_impl(&state).unwrap();
        assert_eq!(out, data.blacklist_templates);
    }

    /// `save_template_impl`：应拒绝空名称或 builtin 模板覆盖。
    #[test]
    fn save_template_rejects_invalid_template() {
        let state = TestState::new(AppData::default());

        let err = save_template_impl(
            &state,
            BlacklistTemplate {
                id: "".to_string(),
                name: "   ".to_string(),
                builtin: false,
                processes: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let err = save_template_impl(
            &state,
            BlacklistTemplate {
                id: "x".to_string(),
                name: "X".to_string(),
                builtin: true,
                processes: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `save_template_impl`：id 为空时应自动生成，且应新增/更新自定义模板。
    #[test]
    fn save_template_generates_id_and_upserts() {
        let state = TestState::new(AppData::default());
        let tpl = BlacklistTemplate {
            id: "".to_string(),
            name: " 自定义 ".to_string(),
            builtin: false,
            processes: vec![BlacklistItem {
                name: "WeChat.exe".to_string(),
                display_name: "微信".to_string(),
            }],
        };

        let saved = save_template_impl(&state, tpl).unwrap();
        assert!(saved.id.starts_with("custom-"));
        assert_eq!(saved.name, "自定义");

        let templates = state.data_snapshot().blacklist_templates;
        assert!(templates.iter().any(|t| t.id == saved.id));

        let updated = save_template_impl(
            &state,
            BlacklistTemplate {
                id: saved.id.clone(),
                name: "自定义2".to_string(),
                builtin: false,
                processes: vec![BlacklistItem {
                    name: "QQ.exe".to_string(),
                    display_name: "QQ".to_string(),
                }],
            },
        )
        .unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.name, "自定义2");
    }

    /// `save_template_impl`：当 id 指向内置模板时应拒绝覆盖（即使 builtin=false）。
    #[test]
    fn save_template_rejects_overwriting_builtin_by_id() {
        let state = TestState::new(AppData::default());
        let err = save_template_impl(
            &state,
            BlacklistTemplate {
                id: "work".to_string(),
                name: "自定义".to_string(),
                builtin: false,
                processes: vec![BlacklistItem {
                    name: "a.exe".to_string(),
                    display_name: "A".to_string(),
                }],
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `delete_template_impl`：应拒绝删除内置模板，并正确删除自定义模板与激活信息。
    #[test]
    fn delete_template_enforces_builtin_and_updates_active_ids() {
        let mut data = AppData::default();
        data.blacklist_templates.push(BlacklistTemplate {
            id: "custom-1".to_string(),
            name: "自定义".to_string(),
            builtin: false,
            processes: vec![BlacklistItem {
                name: "a.exe".to_string(),
                display_name: "A".to_string(),
            }],
        });
        data.active_template_ids = vec!["custom-1".to_string(), "work".to_string()];
        data.active_template_id = Some("custom-1".to_string());
        let state = TestState::new(data);

        let err = delete_template_impl(&state, "work".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let deleted = delete_template_impl(&state, "custom-1".to_string()).unwrap();
        assert!(deleted);
        let snap = state.data_snapshot();
        assert!(!snap.blacklist_templates.iter().any(|t| t.id == "custom-1"));
        assert!(!snap.active_template_ids.iter().any(|x| x == "custom-1"));
        assert_eq!(
            snap.active_template_id.as_deref(),
            snap.active_template_ids.first().map(|s| s.as_str())
        );
    }

    /// `delete_template_impl`：空 id 应返回校验错误。
    #[test]
    fn delete_template_rejects_blank_id() {
        let state = TestState::new(AppData::default());
        let err = delete_template_impl(&state, "   ".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `delete_template_impl`：删除不存在的模板应返回 false（且不报错）。
    #[test]
    fn delete_template_returns_false_when_missing() {
        let state = TestState::new(AppData::default());
        let deleted = delete_template_impl(&state, "missing".to_string()).unwrap();
        assert!(!deleted);
    }

    /// `apply_template_impl`：锁定时禁止切换；未锁定时应切换 active ids 并按进程名去重。
    #[test]
    fn apply_template_toggles_and_dedupes_blacklist() {
        let mut data = AppData::default();
        data.blacklist_templates.push(BlacklistTemplate {
            id: "custom-1".to_string(),
            name: "自定义".to_string(),
            builtin: false,
            processes: vec![
                BlacklistItem {
                    name: "WeChat.exe".to_string(),
                    display_name: "微信".to_string(),
                },
                BlacklistItem {
                    name: "wechat.exe".to_string(),
                    display_name: "微信".to_string(),
                },
            ],
        });
        let state = TestState::new(data);

        // 锁定：模拟工作阶段开始。
        state
            .update_data_and_timer(
                |d, t| {
                    let clock = crate::timer::SystemClock;
                    t.start(&d.settings, &clock);
                    Ok(())
                },
                false,
            )
            .unwrap();
        assert!(state.timer_snapshot().blacklist_locked);

        let err = apply_template_impl(&state, "custom-1".to_string()).unwrap_err();
        assert!(matches!(err, AppError::BlacklistLocked));

        // 解除锁定（跳到休息阶段即可）。
        state
            .update_timer(|t, d| {
                t.skip(&d.settings, 1);
                Ok(())
            })
            .unwrap();
        assert!(!state.timer_snapshot().blacklist_locked);

        // 空 id：应返回校验错误。
        let err = apply_template_impl(&state, "   ".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // 不存在模板：应返回校验错误。
        let err = apply_template_impl(&state, "missing".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // 切换启用 custom-1：应生效，并去重 wechat。
        let out = apply_template_impl(&state, "custom-1".to_string()).unwrap();
        let wechat_count = out
            .iter()
            .filter(|x| normalize_name(&x.name) == "wechat.exe")
            .count();
        assert_eq!(wechat_count, 1);
        assert!(state
            .data_snapshot()
            .active_template_ids
            .iter()
            .any(|x| x == "custom-1"));

        // 再次切换：应关闭 custom-1。
        let _ = apply_template_impl(&state, "custom-1".to_string()).unwrap();
        assert!(!state
            .data_snapshot()
            .active_template_ids
            .iter()
            .any(|x| x == "custom-1"));
    }
}
