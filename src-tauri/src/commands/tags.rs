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
}
