//! 计时器运行态与核心状态机（tick、阶段切换、写入历史）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app_data::{AppData, HistoryDay, HistoryRecord, Phase, Settings};
use crate::errors::AppResult;
use crate::timer::notification;
use crate::timer::stats;

/// 时间来源：用于将状态机与系统时间解耦，便于单元测试。
pub trait TimerClock {
    /// 获取今天日期字符串（YYYY-MM-DD）。
    fn today_date(&self) -> String;
    /// 获取当前时间字符串（HH:mm）。
    fn now_hhmm(&self) -> String;
    /// 获取本周日期范围（周一为起始），返回 `(from, to)`（YYYY-MM-DD）。
    fn current_week_range(&self) -> (String, String);
}

/// 默认时间来源：使用本机时钟（`chrono::Local`）。
pub struct SystemClock;

impl TimerClock for SystemClock {
    /// 获取今天日期字符串（YYYY-MM-DD）。
    fn today_date(&self) -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    /// 获取当前时间字符串（HH:mm）。
    fn now_hhmm(&self) -> String {
        chrono::Local::now().format("%H:%M").to_string()
    }

    /// 获取本周日期范围（周一为起始），返回 `(from, to)`（YYYY-MM-DD）。
    fn current_week_range(&self) -> (String, String) {
        use chrono::{Datelike as _, Duration as ChronoDuration, Weekday};
        let today = chrono::Local::now().date_naive();
        let weekday = today.weekday();
        let offset_days = match weekday {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };
        let from = today - ChronoDuration::days(offset_days);
        let to = from + ChronoDuration::days(6);
        (
            from.format("%Y-%m-%d").to_string(),
            to.format("%Y-%m-%d").to_string(),
        )
    }
}

/// 前端渲染/托盘展示所需的计时器快照。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TimerSnapshot {
    /// 当前阶段。
    pub phase: Phase,
    /// 剩余秒数。
    pub remaining_seconds: u64,
    /// 是否运行中。
    pub is_running: bool,
    /// 当前任务标签。
    pub current_tag: String,
    /// 专注期内黑名单是否锁定（只能增不能减）。
    pub blacklist_locked: bool,
    /// 当前设置（用于前端展示/校验）。
    pub settings: Settings,
    /// 今日统计（用于主界面展示）。
    pub today_stats: stats::TodayStats,
    /// 本周统计（用于主界面展示）。
    pub week_stats: stats::WeekStats,
    /// 目标进度（用于主界面展示与提醒判断）。
    pub goal_progress: stats::GoalProgress,
}

/// tick 结果：用于决定是否需要持久化与是否发生阶段切换。
pub struct TickResult {
    /// 是否写入了历史（需要持久化）。
    pub history_changed: bool,
    /// 是否发生阶段结束切换（用于托盘/通知刷新）。
    pub phase_ended: bool,
    /// 是否在“休息结束”后自动开始了工作阶段（用于触发黑名单终止逻辑）。
    pub work_auto_started: bool,
    /// 若本次 tick 完成了工作阶段，则携带“新记录已写入”的事件负载。
    pub work_completed_event: Option<WorkCompletedEvent>,
}

/// 工作阶段完成事件：用于前端弹出“备注填写”并定位到对应记录。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorkCompletedEvent {
    /// 记录日期（YYYY-MM-DD）。
    pub date: String,
    /// 当日记录索引（从 0 开始）。
    pub record_index: usize,
    /// 写入的记录内容。
    pub record: HistoryRecord,
}

/// 计时器运行态（不持久化；重启后回到默认工作阶段）。
pub struct TimerRuntime {
    /// 当前阶段。
    pub phase: Phase,
    /// 剩余秒数。
    pub remaining_seconds: u64,
    /// 是否运行中。
    pub is_running: bool,
    /// 当前任务标签（用于下一次完成记录）。
    pub current_tag: String,
    /// 工作阶段首次开始时的日期（YYYY-MM-DD）。
    work_started_date: Option<String>,
    /// 工作阶段首次开始时的时间（HH:mm）。
    work_started_time: Option<String>,
    /// 专注期黑名单锁定标记：一旦工作阶段开始，就禁止移除黑名单条目。
    work_lock_active: bool,
    /// 连续番茄“自动推进”剩余工作次数（仅影响：休息结束后是否自动开始工作）。
    auto_work_remaining: u32,
}

impl TimerRuntime {
    /// 构造新的计时器运行态：工作阶段 + `settings.pomodoro`。
    pub fn new(settings: &Settings, tags: &[String], clock: &dyn TimerClock) -> Self {
        Self {
            phase: Phase::Work,
            remaining_seconds: settings.pomodoro as u64 * 60,
            is_running: false,
            current_tag: tags
                .first()
                .cloned()
                .unwrap_or_else(|| "工作".to_string()),
            work_started_date: None,
            work_started_time: None,
            work_lock_active: false,
            auto_work_remaining: 0,
        }
        .with_normalized_tag(clock)
    }

    /// 基于当前数据生成快照（使用系统时钟计算今日/本周统计）。
    pub fn snapshot(&self, data: &AppData) -> TimerSnapshot {
        self.snapshot_with_clock(data, &SystemClock)
    }

    /// 基于当前数据生成快照（可注入 `clock`，用于测试与边界场景）。
    pub fn snapshot_with_clock(&self, data: &AppData, clock: &dyn TimerClock) -> TimerSnapshot {
        let today = clock.today_date();
        let (from, to) = clock.current_week_range();
        let today_stats = stats::compute_today_stats(data, &today);
        let week_stats = stats::compute_week_stats(data, &from, &to);

        TimerSnapshot {
            phase: self.phase,
            remaining_seconds: self.remaining_seconds,
            is_running: self.is_running,
            current_tag: self.current_tag.clone(),
            blacklist_locked: self.blacklist_locked(),
            settings: data.settings.clone(),
            today_stats: today_stats.clone(),
            week_stats: week_stats.clone(),
            goal_progress: stats::GoalProgress {
                daily_goal: data.settings.daily_goal,
                daily_completed: today_stats.total,
                weekly_goal: data.settings.weekly_goal,
                weekly_completed: week_stats.total,
            },
        }
    }

    /// 专注期内黑名单锁定判断。
    pub fn blacklist_locked(&self) -> bool {
        self.phase == Phase::Work && self.work_lock_active
    }

    /// 更新当前标签。
    pub fn set_current_tag(&mut self, tag: String, clock: &dyn TimerClock) {
        self.current_tag = tag;
        *self = std::mem::take(self).with_normalized_tag(clock);
    }

    /// 启动计时；若为工作阶段首次开始则记录开始时间并锁定黑名单。
    pub fn start(&mut self, settings: &Settings, clock: &dyn TimerClock) {
        if self.is_running {
            return;
        }
        self.is_running = true;
        if self.phase == Phase::Work && !self.work_lock_active {
            self.work_lock_active = true;
            self.work_started_date = Some(clock.today_date());
            self.work_started_time = Some(clock.now_hhmm());
            self.init_auto_work_remaining_if_needed(settings);
        }
    }

    /// 暂停计时。
    pub fn pause(&mut self) {
        self.is_running = false;
    }

    /// 重置为工作阶段初始状态（不会清空历史）。
    pub fn reset(&mut self, settings: &Settings) {
        self.phase = Phase::Work;
        self.remaining_seconds = settings.pomodoro as u64 * 60;
        self.is_running = false;
        self.work_started_date = None;
        self.work_started_time = None;
        self.work_lock_active = false;
        self.auto_work_remaining = 0;
    }

    /// 跳过当前阶段（工作阶段不会写入历史）。
    pub fn skip(&mut self, settings: &Settings, completed_today: u32) {
        let next = next_phase(self.phase, settings.long_break_interval, completed_today);
        self.apply_phase(next, settings);
        self.is_running = false;
    }

    /// 每秒 tick：递减剩余时间，并在归零时完成阶段切换与（必要时）写入历史。
    pub fn tick(
        &mut self,
        data: &mut AppData,
        clock: &dyn TimerClock,
        notifier: &dyn notification::Notifier,
    ) -> AppResult<TickResult> {
        if !self.is_running {
            return Ok(TickResult {
                history_changed: false,
                phase_ended: false,
                work_auto_started: false,
                work_completed_event: None,
            });
        }
        if self.remaining_seconds > 0 {
            self.remaining_seconds -= 1;
        }
        if self.remaining_seconds > 0 {
            return Ok(TickResult {
                history_changed: false,
                phase_ended: false,
                work_auto_started: false,
                work_completed_event: None,
            });
        }

        let ended_phase = self.phase;
        let mut history_changed = false;
        let mut work_completed_event: Option<WorkCompletedEvent> = None;

        let today = clock.today_date();
        let (from, to) = clock.current_week_range();
        let mut completed_today_after = stats::compute_today_stats(data, &today).total;
        let completed_today_before = completed_today_after;
        let completed_week_before = stats::compute_week_stats(data, &from, &to).total;
        let mut completed_week_after = completed_week_before;

        if ended_phase == Phase::Work {
            let created = self.append_work_record(data, clock)?;
            history_changed = true;
            completed_today_after += 1;
            completed_week_after += 1;
            self.decrease_auto_work_remaining_after_work_end(&data.settings);
            work_completed_event = Some(created);
            tracing::info!(
                target: "timer",
                "工作阶段完成：date={} tag={} duration={}m todayCompleted={} weekCompleted={}",
                self.work_started_date.clone().unwrap_or_else(|| today.clone()),
                self.current_tag,
                data.settings.pomodoro,
                completed_today_after,
                completed_week_after
            );
            notification::notify_goal_progress_if_needed(
                notifier,
                &data.settings,
                completed_today_before,
                completed_today_after,
                completed_week_before,
                completed_week_after,
            )?;
        }

        let next = next_phase(ended_phase, data.settings.long_break_interval, completed_today_after);
        self.apply_phase(next, &data.settings);
        self.is_running = false;

        let next_auto_started = self.start_next_phase_if_needed(next, &data.settings, clock);

        notification::notify_phase_end(notifier, ended_phase, next, next_auto_started, &data.settings)?;

        tracing::info!(
            target: "timer",
            "阶段切换：ended={:?} next={:?} nextAutoStarted={}",
            ended_phase,
            next,
            next_auto_started
        );

        Ok(TickResult {
            history_changed,
            phase_ended: true,
            work_auto_started: next == Phase::Work && next_auto_started,
            work_completed_event,
        })
    }

    /// 将当前工作阶段写入 `history`（仅在自然完成时调用）。
    fn append_work_record(&mut self, data: &mut AppData, clock: &dyn TimerClock) -> AppResult<WorkCompletedEvent> {
        let date = self
            .work_started_date
            .clone()
            .unwrap_or_else(|| clock.today_date());
        let start_time = self
            .work_started_time
            .clone()
            .unwrap_or_else(|| clock.now_hhmm());
        let end_time = clock.now_hhmm();

        let record = HistoryRecord {
            tag: self.current_tag.clone(),
            start_time,
            end_time: Some(end_time),
            duration: data.settings.pomodoro,
            phase: Phase::Work,
            remark: String::new(),
        };

        let day = ensure_day(&mut data.history, &date);
        day.records.push(record.clone());
        let record_index = day.records.len().saturating_sub(1);
        Ok(WorkCompletedEvent {
            date,
            record_index,
            record,
        })
    }

    /// 应用阶段切换：重置剩余时间与锁定标记。
    fn apply_phase(&mut self, phase: Phase, settings: &Settings) {
        self.phase = phase;
        self.remaining_seconds = phase_seconds(phase, settings);
        self.work_started_date = None;
        self.work_started_time = None;
        self.work_lock_active = false;
    }

    /// 初始化“连续番茄自动推进”的剩余工作次数（仅在工作阶段首次开始时触发）。
    fn init_auto_work_remaining_if_needed(&mut self, settings: &Settings) {
        if !settings.auto_continue_enabled {
            return;
        }
        if self.auto_work_remaining > 0 {
            return;
        }
        self.auto_work_remaining = settings.auto_continue_pomodoros;
    }

    /// 在一个工作阶段自然结束后递减“自动推进”的剩余次数。
    fn decrease_auto_work_remaining_after_work_end(&mut self, settings: &Settings) {
        if !settings.auto_continue_enabled {
            self.auto_work_remaining = 0;
            return;
        }
        if self.auto_work_remaining > 0 {
            self.auto_work_remaining -= 1;
        }
    }

    /// 按规则决定是否自动开始“下一阶段”的倒计时，并返回是否已自动开始。
    fn start_next_phase_if_needed(&mut self, next: Phase, settings: &Settings, clock: &dyn TimerClock) -> bool {
        match next {
            Phase::ShortBreak | Phase::LongBreak => {
                // 工作结束后始终自动进入休息倒计时。
                self.start(settings, clock);
                true
            }
            Phase::Work => {
                // 休息结束后：仅在“连续番茄自动推进”开启且仍有剩余时自动开始工作。
                if settings.auto_continue_enabled && self.auto_work_remaining > 0 {
                    self.start(settings, clock);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 规范化当前标签：用于防御性处理空白标签，保证 UI 与导出稳定。
    fn with_normalized_tag(mut self, clock: &dyn TimerClock) -> Self {
        if self.current_tag.trim().is_empty() {
            self.current_tag = "工作".to_string();
        }
        // 若计时器已经在工作阶段运行中，且日期/时间缺失，则补齐（避免中途迁移导致的 None）。
        if self.phase == Phase::Work && self.is_running {
            if self.work_started_date.is_none() {
                self.work_started_date = Some(clock.today_date());
            }
            if self.work_started_time.is_none() {
                self.work_started_time = Some(clock.now_hhmm());
            }
        }
        self
    }
}

impl Default for TimerRuntime {
    /// 仅用于 `std::mem::take` 的内部占位默认值（不应被业务直接使用）。
    fn default() -> Self {
        Self {
            phase: Phase::Work,
            remaining_seconds: 0,
            is_running: false,
            current_tag: "工作".to_string(),
            work_started_date: None,
            work_started_time: None,
            work_lock_active: false,
            auto_work_remaining: 0,
        }
    }
}

/// 计算某阶段的总秒数。
fn phase_seconds(phase: Phase, settings: &Settings) -> u64 {
    match phase {
        Phase::Work => settings.pomodoro as u64 * 60,
        Phase::ShortBreak => settings.short_break as u64 * 60,
        Phase::LongBreak => settings.long_break as u64 * 60,
    }
}

/// 基于“当前阶段 + 今日已完成番茄数 + 长休息间隔”推导下一阶段。
fn next_phase(current: Phase, long_break_interval: u32, completed_today_after: u32) -> Phase {
    match current {
        Phase::Work => {
            if long_break_interval > 0 && completed_today_after.is_multiple_of(long_break_interval) {
                Phase::LongBreak
            } else {
                Phase::ShortBreak
            }
        }
        Phase::ShortBreak | Phase::LongBreak => Phase::Work,
    }
}

/// 在历史数组中确保存在指定日期的 `HistoryDay`，并返回可变引用。
fn ensure_day<'a>(history: &'a mut Vec<HistoryDay>, date: &str) -> &'a mut HistoryDay {
    if let Some(index) = history.iter().position(|d| d.date == date) {
        return &mut history[index];
    }

    history.push(HistoryDay {
        date: date.to_string(),
        records: Vec::new(),
    });
    let last_index = history.len().saturating_sub(1);
    &mut history[last_index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定时间源：用于让单元测试在任意时间运行都可复现。
    struct FixedClock {
        today: String,
        now: String,
        week_from: String,
        week_to: String,
    }

    impl FixedClock {
        /// 构造一个固定时钟（weekRange 默认覆盖 today）。
        fn new(today: &str, now: &str) -> Self {
            Self {
                today: today.to_string(),
                now: now.to_string(),
                week_from: today.to_string(),
                week_to: today.to_string(),
            }
        }

        /// 覆盖本周范围（闭区间）。
        fn with_week_range(mut self, from: &str, to: &str) -> Self {
            self.week_from = from.to_string();
            self.week_to = to.to_string();
            self
        }
    }

    impl TimerClock for FixedClock {
        /// 返回固定的日期。
        fn today_date(&self) -> String {
            self.today.clone()
        }

        /// 返回固定的时间。
        fn now_hhmm(&self) -> String {
            self.now.clone()
        }

        /// 返回固定的周范围。
        fn current_week_range(&self) -> (String, String) {
            (self.week_from.clone(), self.week_to.clone())
        }
    }

    /// 空通知器：测试时忽略通知副作用。
    struct NoopNotifier;

    impl notification::Notifier for NoopNotifier {
        /// 忽略通知发送并返回成功。
        fn notify(&self, _title: &str, _body: &str) -> AppResult<()> {
            Ok(())
        }
    }

    /// `tick` 在归零时会写入历史并切换到休息阶段。
    #[test]
    fn tick_completes_work_and_switches_to_break() {
        let clock = FixedClock::new("2025-01-01", "09:00").with_week_range("2025-01-01", "2025-01-07");
        let notifier = NoopNotifier;

        let mut data = AppData::default();
        data.settings.pomodoro = 1;
        data.settings.short_break = 1;
        data.settings.long_break = 1;
        data.settings.long_break_interval = 4;
        data.tags = vec!["学习".to_string()];

        let mut runtime = TimerRuntime::new(&data.settings, &data.tags, &clock);
        runtime.start(&data.settings, &clock);
        runtime.remaining_seconds = 1;

        let out = runtime.tick(&mut data, &clock, &notifier).unwrap();
        assert!(out.history_changed);
        assert!(out.phase_ended);
        assert!(out.work_completed_event.is_some());
        assert_eq!(runtime.phase, Phase::ShortBreak);
        assert!(runtime.is_running);
        assert_eq!(runtime.remaining_seconds, (data.settings.short_break as u64) * 60);
        assert_eq!(data.history.len(), 1);
        assert_eq!(data.history[0].records.len(), 1);
    }

    /// 当今日完成数达到长休息间隔倍数时，应进入长休息阶段。
    #[test]
    fn tick_uses_long_break_interval() {
        let clock = FixedClock::new("2025-01-01", "09:00").with_week_range("2025-01-01", "2025-01-07");
        let notifier = NoopNotifier;

        let mut data = AppData::default();
        data.settings.pomodoro = 1;
        data.settings.short_break = 1;
        data.settings.long_break = 2;
        data.settings.long_break_interval = 2;
        data.tags = vec!["A".to_string()];

        // 预置 1 条工作记录：让本次完成后达到 2，从而触发长休息。
        data.history.push(HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "A".to_string(),
                start_time: "08:00".to_string(),
                end_time: Some("08:25".to_string()),
                duration: 25,
                phase: Phase::Work,
                remark: String::new(),
            }],
        });

        let mut runtime = TimerRuntime::new(&data.settings, &data.tags, &clock);
        runtime.start(&data.settings, &clock);
        runtime.remaining_seconds = 1;

        let _ = runtime.tick(&mut data, &clock, &notifier).unwrap();
        assert_eq!(runtime.phase, Phase::LongBreak);
        assert!(runtime.is_running);
        assert_eq!(runtime.remaining_seconds, (data.settings.long_break as u64) * 60);
    }

    /// 休息结束后：未开启连续番茄时不应自动开始工作倒计时。
    #[test]
    fn break_end_does_not_auto_start_work_when_disabled() {
        let clock = FixedClock::new("2025-01-01", "09:00").with_week_range("2025-01-01", "2025-01-07");
        let notifier = NoopNotifier;

        let mut data = AppData::default();
        data.settings.short_break = 1;
        data.settings.auto_continue_enabled = false;
        data.tags = vec!["A".to_string()];

        let mut runtime = TimerRuntime::new(&data.settings, &data.tags, &clock);
        runtime.phase = Phase::ShortBreak;
        runtime.remaining_seconds = 1;
        runtime.is_running = true;

        let out = runtime.tick(&mut data, &clock, &notifier).unwrap();
        assert!(out.phase_ended);
        assert_eq!(runtime.phase, Phase::Work);
        assert!(!runtime.is_running);
    }

    /// 休息结束后：开启连续番茄且仍有剩余时应自动开始工作倒计时。
    #[test]
    fn break_end_auto_starts_work_when_enabled_and_remaining() {
        let clock = FixedClock::new("2025-01-01", "09:00").with_week_range("2025-01-01", "2025-01-07");
        let notifier = NoopNotifier;

        let mut data = AppData::default();
        data.settings.pomodoro = 1;
        data.settings.short_break = 1;
        data.settings.auto_continue_enabled = true;
        data.settings.auto_continue_pomodoros = 2;
        data.tags = vec!["A".to_string()];

        let mut runtime = TimerRuntime::new(&data.settings, &data.tags, &clock);
        runtime.start(&data.settings, &clock);

        // 快速完成一次工作 -> 自动进入短休息并开始。
        runtime.remaining_seconds = 1;
        let _ = runtime.tick(&mut data, &clock, &notifier).unwrap();
        assert_eq!(runtime.phase, Phase::ShortBreak);
        assert!(runtime.is_running);

        // 快速结束短休息 -> 自动开始下一次工作。
        runtime.remaining_seconds = 1;
        let out = runtime.tick(&mut data, &clock, &notifier).unwrap();
        assert!(out.phase_ended);
        assert!(out.work_auto_started);
        assert_eq!(runtime.phase, Phase::Work);
        assert!(runtime.is_running);
    }
}
