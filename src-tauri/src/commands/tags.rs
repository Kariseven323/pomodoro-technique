//! 标签相关命令：设置当前标签、管理标签列表。

use crate::errors::{AppError, AppResult};

use super::state_like::CommandState;
use super::types::AppSnapshot;

/// 设置当前标签的内部实现（便于统一错误处理）。
pub(crate) fn set_current_tag_impl<S: CommandState>(
    state: &S,
    tag: String,
) -> AppResult<AppSnapshot> {
    let clock = crate::timer::SystemClock;
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }

    state.update_data_and_timer(
        |data, timer_runtime| {
            timer_runtime.set_current_tag(tag.clone(), &clock);
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

/// 新增标签的内部实现（便于统一错误处理）。
pub(crate) fn add_tag_impl<S: CommandState>(state: &S, tag: String) -> AppResult<Vec<String>> {
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

/// 重命名标签的内部实现：同步更新 tags 列表、计时器当前标签与历史记录。
pub(crate) fn rename_tag_impl<S: CommandState>(
    state: &S,
    from: String,
    to: String,
) -> AppResult<AppSnapshot> {
    let clock = crate::timer::SystemClock;
    let from = from.trim().to_string();
    let to = to.trim().to_string();
    if from.is_empty() || to.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }
    if from == to {
        return Ok(AppSnapshot {
            data: state.data_snapshot(),
            timer: state.timer_snapshot(),
        });
    }

    state.update_data_and_timer(
        |data, timer_runtime| {
            if !data.tags.iter().any(|t| t == &from) {
                return Err(AppError::Validation("原标签不存在".to_string()));
            }
            if data.tags.iter().any(|t| t == &to) {
                return Err(AppError::Validation("目标标签已存在".to_string()));
            }

            for t in data.tags.iter_mut() {
                if *t == from {
                    *t = to.clone();
                }
            }

            for day in data.history.iter_mut() {
                for r in day.records.iter_mut() {
                    if r.tag == from {
                        r.tag = to.clone();
                    }
                }
            }
            for day in data.history_dev.iter_mut() {
                for r in day.records.iter_mut() {
                    if r.tag == from {
                        r.tag = to.clone();
                    }
                }
            }
            for d in data.interruptions.iter_mut() {
                for r in d.records.iter_mut() {
                    if r.tag == from {
                        r.tag = to.clone();
                    }
                }
            }

            if timer_runtime.current_tag == from {
                timer_runtime.set_current_tag(to.clone(), &clock);
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

/// 删除标签的内部实现：同步更新 tags 列表、计时器当前标签与历史记录。
pub(crate) fn delete_tag_impl<S: CommandState>(state: &S, tag: String) -> AppResult<AppSnapshot> {
    let clock = crate::timer::SystemClock;
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err(AppError::Validation("标签不能为空".to_string()));
    }
    if tag == "工作" {
        return Err(AppError::Validation("默认标签不可删除".to_string()));
    }

    state.update_data_and_timer(
        |data, timer_runtime| {
            if !data.tags.iter().any(|t| t == &tag) {
                return Err(AppError::Validation("标签不存在".to_string()));
            }
            data.tags.retain(|t| t != &tag);

            for day in data.history.iter_mut() {
                for r in day.records.iter_mut() {
                    if r.tag == tag {
                        r.tag = "".to_string();
                    }
                }
            }
            for day in data.history_dev.iter_mut() {
                for r in day.records.iter_mut() {
                    if r.tag == tag {
                        r.tag = "".to_string();
                    }
                }
            }
            for d in data.interruptions.iter_mut() {
                for r in d.records.iter_mut() {
                    if r.tag == tag {
                        r.tag = "".to_string();
                    }
                }
            }

            if timer_runtime.current_tag == tag {
                timer_runtime.set_current_tag("工作".to_string(), &clock);
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::AppData;
    use crate::commands::state_like::TestState;

    /// `set_current_tag_impl`：空白标签应被拒绝。
    #[test]
    fn set_current_tag_rejects_blank() {
        let state = TestState::new(AppData::default());
        let err = set_current_tag_impl(&state, "   ".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `set_current_tag_impl`：应更新计时器当前标签，并在必要时追加到 tags。
    #[test]
    fn set_current_tag_updates_timer_and_tags() {
        let state = TestState::new(AppData::default());
        let snapshot = set_current_tag_impl(&state, "新标签".to_string()).unwrap();
        assert_eq!(snapshot.timer.current_tag, "新标签");
        assert!(snapshot.data.tags.iter().any(|t| t == "新标签"));
        assert!(state.emitted_timer_snapshot_count() >= 1);
    }

    /// `add_tag_impl`：应 trim 并去重追加到 tags。
    #[test]
    fn add_tag_trims_and_dedupes() {
        let state = TestState::new(AppData::default());
        let tags = add_tag_impl(&state, "  新标签  ".to_string()).unwrap();
        assert!(tags.iter().any(|t| t == "新标签"));

        let tags2 = add_tag_impl(&state, "新标签".to_string()).unwrap();
        let count = tags2.iter().filter(|t| *t == "新标签").count();
        assert_eq!(count, 1);
    }

    /// `add_tag_impl`：空白标签应被拒绝。
    #[test]
    fn add_tag_rejects_blank() {
        let state = TestState::new(AppData::default());
        let err = add_tag_impl(&state, "   ".to_string()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// `rename_tag_impl`：应同步更新 tags、timer.current_tag 与历史记录 tag。
    #[test]
    fn rename_tag_updates_tags_timer_and_history() {
        let mut data = AppData::default();
        data.tags = vec!["工作".to_string(), "旧".to_string()];
        data.history = vec![crate::app_data::HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![crate::app_data::HistoryRecord {
                tag: "旧".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: crate::app_data::Phase::Work,
                remark: "".to_string(),
            }],
        }];
        let state = TestState::new(data);

        let snapshot = rename_tag_impl(&state, "旧".to_string(), "新".to_string()).unwrap();
        assert!(snapshot.data.tags.iter().any(|t| t == "新"));
        assert!(!snapshot.data.tags.iter().any(|t| t == "旧"));
        assert_eq!(snapshot.data.history[0].records[0].tag, "新");
        assert!(state.emitted_timer_snapshot_count() >= 1);
    }

    /// `delete_tag_impl`：应从 tags 移除，并清空历史记录中的该标签。
    #[test]
    fn delete_tag_removes_and_clears_history() {
        let mut data = AppData::default();
        data.tags = vec!["工作".to_string(), "A".to_string()];
        data.history = vec![crate::app_data::HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![crate::app_data::HistoryRecord {
                tag: "A".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: crate::app_data::Phase::Work,
                remark: "".to_string(),
            }],
        }];
        let state = TestState::new(data);

        let snapshot = delete_tag_impl(&state, "A".to_string()).unwrap();
        assert!(!snapshot.data.tags.iter().any(|t| t == "A"));
        assert_eq!(snapshot.data.history[0].records[0].tag, "");
        assert!(state.emitted_timer_snapshot_count() >= 1);
    }
}
