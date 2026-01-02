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
