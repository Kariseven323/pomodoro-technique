//! 计时器引擎：阶段切换、倒计时、历史记录与通知触发。

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::Manager as _;
use tauri_plugin_notification::NotificationExt as _;
use tokio::time::sleep;

use crate::app_data::{AppData, HistoryDay, HistoryRecord, Phase, Settings};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

/// 向前端广播计时器状态的事件名。
pub const EVENT_SNAPSHOT: &str = "pomodoro://snapshot";

/// 标签计数条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    /// 标签名。
    pub tag: String,
    /// 完成次数。
    pub count: u32,
}

/// 今日统计（总数 + 按标签分组）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayStats {
    /// 今日完成的番茄总数。
    pub total: u32,
    /// 按标签统计。
    pub by_tag: Vec<TagCount>,
}

impl TodayStats {
    /// 从持久化 `AppData` 计算当天统计。
    pub fn from_app_data(data: &AppData) -> Self {
        let today = today_date();
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        let mut total = 0u32;

        if let Some(day) = data.history.iter().find(|d| d.date == today) {
            for r in &day.records {
                total += 1;
                *map.entry(r.tag.clone()).or_insert(0) += 1;
            }
        }

        Self {
            total,
            by_tag: map
                .into_iter()
                .map(|(tag, count)| TagCount { tag, count })
                .collect(),
        }
    }
}

/// 本周统计（总数 + 按标签分组）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekStats {
    /// 本周完成的番茄总数。
    pub total: u32,
    /// 按标签统计。
    pub by_tag: Vec<TagCount>,
}

impl WeekStats {
    /// 从持久化 `AppData` 计算本周统计（周一为一周起始）。
    pub fn from_app_data(data: &AppData) -> Self {
        let (from, to) = current_week_range();
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        let mut total = 0u32;

        for day in &data.history {
            if day.date < from || day.date > to {
                continue;
            }
            for r in &day.records {
                total += 1;
                *map.entry(r.tag.clone()).or_insert(0) += 1;
            }
        }

        Self {
            total,
            by_tag: map
                .into_iter()
                .map(|(tag, count)| TagCount { tag, count })
                .collect(),
        }
    }
}

/// 目标进度（每日/每周）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalProgress {
    /// 每日目标（0 表示未设置）。
    pub daily_goal: u32,
    /// 今日已完成。
    pub daily_completed: u32,
    /// 每周目标（0 表示未设置）。
    pub weekly_goal: u32,
    /// 本周已完成。
    pub weekly_completed: u32,
}

/// 前端渲染/托盘展示所需的计时器快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub today_stats: TodayStats,
    /// 本周统计（用于主界面展示）。
    pub week_stats: WeekStats,
    /// 目标进度（用于主界面展示与提醒判断）。
    pub goal_progress: GoalProgress,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCompletedEvent {
    /// 记录日期（YYYY-MM-DD）。
    pub date: String,
    /// 当日记录索引（从 0 开始）。
    pub record_index: usize,
    /// 写入的记录内容。
    pub record: HistoryRecord,
}

/// 向前端广播“工作阶段自然完成”的事件名。
pub const EVENT_WORK_COMPLETED: &str = "pomodoro://work_completed";

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
    pub fn new(settings: &Settings, tags: &[String]) -> Self {
        Self {
            phase: Phase::Work,
            remaining_seconds: settings.pomodoro as u64 * 60,
            is_running: false,
            current_tag: tags.first().cloned().unwrap_or_else(|| "工作".to_string()),
            work_started_date: None,
            work_started_time: None,
            work_lock_active: false,
            auto_work_remaining: 0,
        }
    }

    /// 基于当前数据生成快照。
    pub fn snapshot(&self, data: &AppData) -> TimerSnapshot {
        let today_stats = TodayStats::from_app_data(data);
        let week_stats = WeekStats::from_app_data(data);
        TimerSnapshot {
            phase: self.phase,
            remaining_seconds: self.remaining_seconds,
            is_running: self.is_running,
            current_tag: self.current_tag.clone(),
            blacklist_locked: self.blacklist_locked(),
            settings: data.settings.clone(),
            today_stats: today_stats.clone(),
            week_stats: week_stats.clone(),
            goal_progress: GoalProgress {
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
    pub fn set_current_tag(&mut self, tag: String) {
        self.current_tag = tag;
    }

    /// 启动计时；若为工作阶段首次开始则记录开始时间并锁定黑名单。
    pub fn start(&mut self, settings: &Settings) {
        if self.is_running {
            return;
        }
        self.is_running = true;
        if self.phase == Phase::Work && !self.work_lock_active {
            self.work_lock_active = true;
            self.work_started_date = Some(today_date());
            self.work_started_time = Some(now_hhmm());
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
    pub fn tick(&mut self, data: &mut AppData, app: &tauri::AppHandle) -> AppResult<TickResult> {
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
        let mut completed_today_after = TodayStats::from_app_data(data).total;
        let completed_today_before = completed_today_after;
        let completed_week_before = WeekStats::from_app_data(data).total;
        let mut completed_week_after = completed_week_before;

        if ended_phase == Phase::Work {
            let created = self.append_work_record(data)?;
            history_changed = true;
            completed_today_after += 1;
            completed_week_after += 1;
            self.decrease_auto_work_remaining_after_work_end(&data.settings);
            work_completed_event = Some(created);
            tracing::info!(
                target: "timer",
                "工作阶段完成：date={} tag={} duration={}m todayCompleted={} weekCompleted={}",
                self.work_started_date.clone().unwrap_or_else(today_date),
                self.current_tag,
                data.settings.pomodoro,
                completed_today_after,
                completed_week_after
            );
            self.notify_goal_progress_if_needed(
                app,
                &data.settings,
                completed_today_before,
                completed_today_after,
                completed_week_before,
                completed_week_after,
            )?;
        }

        let next = next_phase(
            ended_phase,
            data.settings.long_break_interval,
            completed_today_after,
        );
        self.apply_phase(next, &data.settings);
        self.is_running = false;

        let next_auto_started = self.start_next_phase_if_needed(next, &data.settings);

        self.notify_phase_end(app, ended_phase, next, next_auto_started, &data.settings)?;

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
    fn append_work_record(&mut self, data: &mut AppData) -> AppResult<WorkCompletedEvent> {
        let date = self.work_started_date.clone().unwrap_or_else(today_date);
        let start_time = self.work_started_time.clone().unwrap_or_else(now_hhmm);
        let end_time = now_hhmm();

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
    fn start_next_phase_if_needed(&mut self, next: Phase, settings: &Settings) -> bool {
        match next {
            Phase::ShortBreak | Phase::LongBreak => {
                // 工作结束后始终自动进入休息倒计时。
                self.start(settings);
                true
            }
            Phase::Work => {
                // 休息结束后：仅在“连续番茄自动推进”开启且仍有剩余时自动开始工作。
                if settings.auto_continue_enabled && self.auto_work_remaining > 0 {
                    self.start(settings);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 发送阶段结束通知，并给出下一阶段预告。
    fn notify_phase_end(
        &self,
        app: &tauri::AppHandle,
        ended: Phase,
        next: Phase,
        next_auto_started: bool,
        settings: &Settings,
    ) -> AppResult<()> {
        let preview = phase_preview(next, next_auto_started, settings);
        let (title, body) = match ended {
            Phase::Work => (
                "专注完成".to_string(),
                format!("{}。{}", "本阶段已结束", preview),
            ),
            Phase::ShortBreak => ("短休息结束".to_string(), preview),
            Phase::LongBreak => ("长休息结束".to_string(), preview),
        };

        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()?;
        Ok(())
    }

    /// 在工作阶段完成后，根据每日/每周目标的阈值触发提醒。
    fn notify_goal_progress_if_needed(
        &self,
        app: &tauri::AppHandle,
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
                app.notification()
                    .builder()
                    .title("今日目标进度")
                    .body(format!("已完成今日目标 50%（{daily_after}/{daily_goal}）"))
                    .show()?;
            }
            if daily_before < daily_goal && daily_after >= daily_goal {
                app.notification()
                    .builder()
                    .title("今日目标达成")
                    .body(format!(
                        "恭喜！已完成今日目标（{daily_after}/{daily_goal}）"
                    ))
                    .show()?;
            }
        }

        let weekly_goal = settings.weekly_goal;
        if weekly_goal > 0 {
            let half = weekly_goal.div_ceil(2);
            if weekly_before < half && weekly_after >= half {
                app.notification()
                    .builder()
                    .title("本周目标进度")
                    .body(format!(
                        "已完成本周目标 50%（{weekly_after}/{weekly_goal}）"
                    ))
                    .show()?;
            }
            if weekly_before < weekly_goal && weekly_after >= weekly_goal {
                app.notification()
                    .builder()
                    .title("本周目标达成")
                    .body(format!(
                        "恭喜！已完成本周目标（{weekly_after}/{weekly_goal}）"
                    ))
                    .show()?;
            }
        }

        Ok(())
    }
}

/// 启动后台 tick 任务：每秒更新计时器、推送事件与刷新托盘。
pub fn spawn_timer_task(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            let state = app.state::<AppState>();
            let was_running = state.is_running();
            if let Ok(result) = state.tick() {
                if result.work_auto_started {
                    let names: Vec<String> = state
                        .data_snapshot()
                        .blacklist
                        .into_iter()
                        .map(|b| b.name)
                        .collect();
                    let payload = crate::processes::kill_names_best_effort(&names);
                    let _ = state.emit_kill_result(payload);
                }
                if was_running || result.phase_ended {
                    let _ = state.emit_timer_snapshot();
                    let _ = crate::tray::refresh_tray(&state);
                }
            }
        }
    });
}

/// 获取今天日期字符串（YYYY-MM-DD）。
fn today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 获取当前时间字符串（HH:mm）。
fn now_hhmm() -> String {
    Local::now().format("%H:%M").to_string()
}

/// 获取本周日期范围（周一为起始），返回 `(from, to)` 的日期字符串（YYYY-MM-DD）。
fn current_week_range() -> (String, String) {
    use chrono::{Datelike as _, Duration as ChronoDuration, Weekday};
    let today = Local::now().date_naive();
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
            if long_break_interval > 0 && completed_today_after.is_multiple_of(long_break_interval)
            {
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

/// 生成“下一阶段预告”文案（区分是否已自动开始）。
fn phase_preview(next: Phase, next_auto_started: bool, settings: &Settings) -> String {
    let prefix = if next_auto_started {
        "已自动开始"
    } else {
        "即将开始"
    };
    match next {
        Phase::Work => format!("{prefix}工作 {} 分钟", settings.pomodoro),
        Phase::ShortBreak => format!("{prefix}短休息 {} 分钟", settings.short_break),
        Phase::LongBreak => format!("{prefix}长休息 {} 分钟", settings.long_break),
    }
}

/// 校验并规范化设置参数（同时满足 PRD 的范围约束）。
pub fn validate_settings(settings: &Settings) -> AppResult<()> {
    if !(1..=60).contains(&settings.pomodoro) {
        return Err(AppError::Validation("番茄时长需在 1-60 分钟".to_string()));
    }
    if !(1..=30).contains(&settings.short_break) {
        return Err(AppError::Validation("短休息需在 1-30 分钟".to_string()));
    }
    if !(1..=60).contains(&settings.long_break) {
        return Err(AppError::Validation("长休息需在 1-60 分钟".to_string()));
    }
    if !(1..=10).contains(&settings.long_break_interval) {
        return Err(AppError::Validation(
            "长休息间隔需在 1-10 个番茄".to_string(),
        ));
    }
    if !(1..=20).contains(&settings.auto_continue_pomodoros) {
        return Err(AppError::Validation(
            "连续番茄数量需在 1-20 个番茄".to_string(),
        ));
    }
    if settings.daily_goal > 1000 {
        return Err(AppError::Validation("每日目标建议不超过 1000".to_string()));
    }
    if settings.weekly_goal > 10000 {
        return Err(AppError::Validation("每周目标建议不超过 10000".to_string()));
    }
    Ok(())
}
