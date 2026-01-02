//! 阶段结束通知与目标达成提醒（通过可注入 Notifier 实现，便于测试）。

use crate::app_data::{Phase, Settings};
use crate::errors::AppResult;

/// 通知发送抽象：用于将“通知内容生成”与“通知实现（Tauri/其它）”解耦。
pub trait Notifier {
    /// 发送一条系统通知。
    fn notify(&self, title: &str, body: &str) -> AppResult<()>;
}

/// Tauri 通知实现（基于 `tauri-plugin-notification`）。
pub struct TauriNotifier<'a> {
    app: &'a tauri::AppHandle,
}

impl<'a> TauriNotifier<'a> {
    /// 创建一个基于 `AppHandle` 的通知器。
    pub fn new(app: &'a tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl Notifier for TauriNotifier<'_> {
    /// 发送系统通知（失败时返回 `AppError::Notification`）。
    fn notify(&self, title: &str, body: &str) -> AppResult<()> {
        use tauri_plugin_notification::NotificationExt as _;
        self.app.notification().builder().title(title).body(body).show()?;
        Ok(())
    }
}

/// 发送阶段结束通知，并给出下一阶段预告。
pub fn notify_phase_end(
    notifier: &dyn Notifier,
    ended: Phase,
    next: Phase,
    next_auto_started: bool,
    settings: &Settings,
) -> AppResult<()> {
    let preview = phase_preview(next, next_auto_started, settings);
    let (title, body) = match ended {
        Phase::Work => ("专注完成".to_string(), format!("{}。{}", "本阶段已结束", preview)),
        Phase::ShortBreak => ("短休息结束".to_string(), preview),
        Phase::LongBreak => ("长休息结束".to_string(), preview),
    };

    notifier.notify(&title, &body)?;
    Ok(())
}

/// 在工作阶段完成后，根据每日/每周目标的阈值触发提醒。
pub fn notify_goal_progress_if_needed(
    notifier: &dyn Notifier,
    settings: &Settings,
    daily_before: u32,
    daily_after: u32,
    weekly_before: u32,
    weekly_after: u32,
) -> AppResult<()> {
    let daily_goal = settings.daily_goal;
    if daily_goal > 0 {
        let half = daily_goal.div_ceil(2);
        if daily_before < half && daily_after >= half {
            notifier.notify("今日目标进度", &format!("已完成今日目标 50%（{daily_after}/{daily_goal}）"))?;
        }
        if daily_before < daily_goal && daily_after >= daily_goal {
            notifier.notify("今日目标达成", &format!("恭喜！已完成今日目标（{daily_after}/{daily_goal}）"))?;
        }
    }

    let weekly_goal = settings.weekly_goal;
    if weekly_goal > 0 {
        let half = weekly_goal.div_ceil(2);
        if weekly_before < half && weekly_after >= half {
            notifier.notify("本周目标进度", &format!("已完成本周目标 50%（{weekly_after}/{weekly_goal}）"))?;
        }
        if weekly_before < weekly_goal && weekly_after >= weekly_goal {
            notifier.notify("本周目标达成", &format!("恭喜！已完成本周目标（{weekly_after}/{weekly_goal}）"))?;
        }
    }

    Ok(())
}

/// 生成“下一阶段预告”文案（区分是否已自动开始）。
fn phase_preview(next: Phase, next_auto_started: bool, settings: &Settings) -> String {
    let prefix = if next_auto_started { "已自动开始" } else { "即将开始" };
    match next {
        Phase::Work => format!("{prefix}工作 {} 分钟", settings.pomodoro),
        Phase::ShortBreak => format!("{prefix}短休息 {} 分钟", settings.short_break),
        Phase::LongBreak => format!("{prefix}长休息 {} 分钟", settings.long_break),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;

    /// 记录型通知器：将所有通知内容收集起来，便于断言。
    struct RecordingNotifier {
        calls: RefCell<Vec<(String, String)>>,
    }

    impl RecordingNotifier {
        /// 创建一个空的记录型通知器。
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
            }
        }

        /// 取出已记录的通知（按调用顺序）。
        fn take(&self) -> Vec<(String, String)> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }
    }

    impl Notifier for RecordingNotifier {
        /// 记录通知内容并返回成功。
        fn notify(&self, title: &str, body: &str) -> AppResult<()> {
            self.calls
                .borrow_mut()
                .push((title.to_string(), body.to_string()));
            Ok(())
        }
    }

    /// `phase_preview`：应按阶段输出时长，并区分“自动开始/即将开始”前缀。
    #[test]
    fn phase_preview_formats_for_all_phases() {
        let settings = Settings {
            pomodoro: 25,
            short_break: 5,
            long_break: 15,
            ..Settings::default()
        };

        assert_eq!(
            phase_preview(Phase::Work, false, &settings),
            "即将开始工作 25 分钟"
        );
        assert_eq!(
            phase_preview(Phase::ShortBreak, false, &settings),
            "即将开始短休息 5 分钟"
        );
        assert_eq!(
            phase_preview(Phase::LongBreak, true, &settings),
            "已自动开始长休息 15 分钟"
        );
    }

    /// `notify_phase_end`：工作/短休息/长休息结束应发送对应标题与预告文案。
    #[test]
    fn notify_phase_end_sends_expected_titles_and_bodies() {
        let notifier = RecordingNotifier::new();
        let settings = Settings {
            pomodoro: 25,
            short_break: 5,
            long_break: 15,
            ..Settings::default()
        };

        notify_phase_end(
            &notifier,
            Phase::Work,
            Phase::ShortBreak,
            false,
            &settings,
        )
        .unwrap();
        notify_phase_end(
            &notifier,
            Phase::ShortBreak,
            Phase::Work,
            true,
            &settings,
        )
        .unwrap();
        notify_phase_end(
            &notifier,
            Phase::LongBreak,
            Phase::Work,
            false,
            &settings,
        )
        .unwrap();

        let calls = notifier.take();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "专注完成");
        assert_eq!(calls[0].1, "本阶段已结束。即将开始短休息 5 分钟");
        assert_eq!(calls[1].0, "短休息结束");
        assert_eq!(calls[1].1, "已自动开始工作 25 分钟");
        assert_eq!(calls[2].0, "长休息结束");
        assert_eq!(calls[2].1, "即将开始工作 25 分钟");
    }

    /// `notify_goal_progress_if_needed`：当目标为 0 时不应发送任何通知。
    #[test]
    fn notify_goal_progress_skips_when_goals_are_zero() {
        let notifier = RecordingNotifier::new();
        let settings = Settings {
            daily_goal: 0,
            weekly_goal: 0,
            ..Settings::default()
        };

        notify_goal_progress_if_needed(&notifier, &settings, 0, 100, 0, 100).unwrap();
        assert!(notifier.take().is_empty());
    }

    /// `notify_goal_progress_if_needed`：应在跨过 50% 与 100% 阈值时发送提醒（含一次跨多个阈值）。
    #[test]
    fn notify_goal_progress_sends_threshold_notifications() {
        let notifier = RecordingNotifier::new();
        let settings = Settings {
            daily_goal: 5,  // half = 3
            weekly_goal: 4, // half = 2
            ..Settings::default()
        };

        // 日目标：2 -> 5 同时跨过 50% 与 100%；周目标：1 -> 4 同时跨过 50% 与 100%。
        notify_goal_progress_if_needed(&notifier, &settings, 2, 5, 1, 4).unwrap();

        let calls = notifier.take();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].0, "今日目标进度");
        assert!(calls[0].1.contains("50%（5/5）"));
        assert_eq!(calls[1].0, "今日目标达成");
        assert!(calls[1].1.contains("（5/5）"));
        assert_eq!(calls[2].0, "本周目标进度");
        assert!(calls[2].1.contains("50%（4/4）"));
        assert_eq!(calls[3].0, "本周目标达成");
        assert!(calls[3].1.contains("（4/4）"));
    }

    /// `notify_goal_progress_if_needed`：未跨过阈值时不应重复提醒（幂等）。
    #[test]
    fn notify_goal_progress_is_idempotent_when_not_crossing() {
        let notifier = RecordingNotifier::new();
        let settings = Settings {
            daily_goal: 4, // half = 2
            weekly_goal: 0,
            ..Settings::default()
        };

        notify_goal_progress_if_needed(&notifier, &settings, 2, 2, 0, 0).unwrap();
        assert!(notifier.take().is_empty());

        notify_goal_progress_if_needed(&notifier, &settings, 3, 3, 0, 0).unwrap();
        assert!(notifier.take().is_empty());
    }
}
