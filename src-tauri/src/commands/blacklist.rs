//! 黑名单相关命令：设置黑名单、专注期锁定校验等。

use crate::app_data::BlacklistItem;
use crate::errors::{AppError, AppResult};

use super::state_like::CommandState;
use super::validation::{normalize_name, validate_blacklist_items};

/// 设置黑名单的内部实现（便于统一错误处理）。
pub(crate) fn set_blacklist_impl<S: CommandState>(
    state: &S,
    blacklist: Vec<BlacklistItem>,
) -> AppResult<Vec<BlacklistItem>> {
    set_blacklist_impl_with_killer(state, blacklist, |names| {
        crate::processes::kill_names_best_effort(names)
    })
}

/// 设置黑名单的可注入实现：用于在测试中 mock 进程终止逻辑。
fn set_blacklist_impl_with_killer<S: CommandState>(
    state: &S,
    blacklist: Vec<BlacklistItem>,
    kill_names: impl FnOnce(&[String]) -> crate::processes::KillSummary,
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
        let payload = kill_names(&added_names);
        let _ = state.emit_kill_result(payload);
    }

    Ok(state.data_snapshot().blacklist)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::commands::state_like::CommandState;
    use crate::commands::state_like::TestState;

    /// `set_blacklist_impl_with_killer`：非锁定状态下允许增删，且不触发终止逻辑。
    #[test]
    fn set_blacklist_allows_replace_when_not_locked() {
        let state = TestState::new(crate::app_data::AppData::default());
        let blacklist = vec![BlacklistItem {
            name: "a.exe".to_string(),
            display_name: "A".to_string(),
        }];

        let out = set_blacklist_impl_with_killer(&state, blacklist.clone(), |_names| {
            crate::processes::KillSummary {
                items: Vec::new(),
                requires_admin: false,
            }
        })
        .unwrap();

        assert_eq!(out, blacklist);
        assert_eq!(state.data_snapshot().blacklist, blacklist);
        assert!(state.take_kill_results().is_empty());
    }

    /// `set_blacklist_impl`：在非锁定状态下应可正常写入（并复用真实 killer，但不会触发系统调用）。
    #[test]
    fn set_blacklist_impl_is_safe_when_not_locked() {
        let state = TestState::new(crate::app_data::AppData::default());
        let blacklist = vec![BlacklistItem {
            name: "a.exe".to_string(),
            display_name: "A".to_string(),
        }];

        let out = set_blacklist_impl(&state, blacklist.clone()).unwrap();
        assert_eq!(out, blacklist);
        assert!(state.take_kill_results().is_empty());
    }

    /// `set_blacklist_impl`：非法黑名单（如空名称）应返回校验错误。
    #[test]
    fn set_blacklist_impl_rejects_invalid_items() {
        let state = TestState::new(crate::app_data::AppData::default());
        let err = set_blacklist_impl(
            &state,
            vec![BlacklistItem {
                name: "   ".to_string(),
                display_name: "A".to_string(),
            }],
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `set_blacklist_impl_with_killer`：锁定状态下禁止移除旧条目，但允许新增并触发 best-effort 终止。
    #[test]
    fn set_blacklist_enforces_lock_and_kills_added_items() {
        let state = TestState::new(crate::app_data::AppData::default());

        // 先设置一个旧黑名单。
        let old = vec![BlacklistItem {
            name: "old.exe".to_string(),
            display_name: "Old".to_string(),
        }];
        set_blacklist_impl_with_killer(&state, old.clone(), |_names| {
            crate::processes::KillSummary {
                items: Vec::new(),
                requires_admin: false,
            }
        })
        .unwrap();

        // 进入“工作阶段锁定”态：让 timer_runtime.blacklist_locked() 为 true。
        state
            .update_data_and_timer(
                |data, timer_runtime| {
                    let clock = crate::timer::SystemClock;
                    timer_runtime.start(&data.settings, &clock);
                    Ok(())
                },
                false,
            )
            .unwrap();
        assert!(state.timer_snapshot().blacklist_locked);

        // 尝试移除 old：应失败。
        let err = set_blacklist_impl_with_killer(&state, vec![], |_names| {
            crate::processes::KillSummary {
                items: Vec::new(),
                requires_admin: false,
            }
        })
        .unwrap_err();
        assert!(matches!(err, AppError::BlacklistLocked));

        // 新增一条：允许，并应触发 kill。
        let next = vec![
            old[0].clone(),
            BlacklistItem {
                name: "new.exe".to_string(),
                display_name: "New".to_string(),
            },
        ];
        let out = set_blacklist_impl_with_killer(&state, next.clone(), |names| {
            crate::processes::KillSummary {
                items: vec![crate::processes::termination::KillItem {
                    name: names[0].clone(),
                    pids: vec![1],
                    killed: 1,
                    failed: 0,
                    requires_admin: false,
                }],
                requires_admin: false,
            }
        })
        .unwrap();

        assert_eq!(out, next);
        let kills = state.take_kill_results();
        assert_eq!(kills.len(), 1);
        assert_eq!(kills[0].items[0].name, "new.exe");
    }
}
