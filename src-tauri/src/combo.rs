//! Combo 连击机制：在满足“休息时长 + 5 分钟”窗口内连续完成时累加。

use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::app_data::{AppData, Phase, Settings};
use crate::errors::{AppError, AppResult};
use crate::timer::TimerClock;

/// Combo 判定额外宽限（分钟）。
const COMBO_GRACE_MINUTES: i64 = 5;

/// Combo 运行态：用于在“工作开始/工作完成/中断”之间保存必要的上下文。
#[derive(Debug, Clone)]
pub struct ComboRuntime {
    /// 最近一次“工作阶段完成”的时间点（本地日期 + HH:mm）。
    last_completed_at: Option<NaiveDateTime>,
    /// 最近一次完成后“预期休息时长”（分钟；短休/长休）。
    last_expected_break_minutes: u32,
    /// 当前工作是否属于“连续开始”（由工作开始时判断，供完成时累加使用）。
    current_work_is_continuation: bool,
}

impl Default for ComboRuntime {
    /// 默认：无历史完成点，Combo 未开始。
    fn default() -> Self {
        Self {
            last_completed_at: None,
            last_expected_break_minutes: 0,
            current_work_is_continuation: false,
        }
    }
}

impl ComboRuntime {
    /// 创建默认的 Combo 运行态。
    pub fn new() -> Self {
        Self::default()
    }

    /// 在“工作阶段首次开始”时更新 `current_work_is_continuation` 标记。
    pub fn on_work_started(&mut self, clock: &dyn TimerClock) -> AppResult<()> {
        let now = clock_naive_now(clock)?;
        let Some(last) = self.last_completed_at else {
            self.current_work_is_continuation = false;
            return Ok(());
        };

        let diff = now - last;
        let allowed_minutes = i64::from(self.last_expected_break_minutes) + COMBO_GRACE_MINUTES;
        self.current_work_is_continuation =
            diff.num_minutes() >= 0 && diff <= ChronoDuration::minutes(allowed_minutes);
        Ok(())
    }

    /// 在“工作阶段自然完成”时更新 `AppData.current_combo`，并记录下一次 Combo 的判定窗口。
    pub fn on_work_completed(
        &mut self,
        data: &mut AppData,
        clock: &dyn TimerClock,
        expected_break: Phase,
        settings: &Settings,
    ) -> AppResult<u32> {
        let next_combo = if data.current_combo == 0 {
            1
        } else if self.current_work_is_continuation {
            data.current_combo.saturating_add(1)
        } else {
            1
        };

        data.current_combo = next_combo;
        self.current_work_is_continuation = false;
        self.last_completed_at = Some(clock_naive_now(clock)?);
        self.last_expected_break_minutes = match expected_break {
            Phase::ShortBreak => settings.short_break,
            Phase::LongBreak => settings.long_break,
            Phase::Work => 0,
        };

        Ok(next_combo)
    }

    /// 在“工作阶段中断”时清空 Combo（PRD v4：中断后 Combo 重置为 0）。
    pub fn on_interrupted(&mut self, data: &mut AppData) {
        data.current_combo = 0;
        self.last_completed_at = None;
        self.last_expected_break_minutes = 0;
        self.current_work_is_continuation = false;
    }
}

/// 将 `TimerClock` 的日期/时间字符串解析为 `NaiveDateTime`（分钟精度）。
fn clock_naive_now(clock: &dyn TimerClock) -> AppResult<NaiveDateTime> {
    let ymd = clock.today_date();
    let hhmm = clock.now_hhmm();

    let date = NaiveDate::parse_from_str(&ymd, "%Y-%m-%d")
        .map_err(|_| AppError::Invariant(format!("TimerClock.today_date 返回非法日期：{ymd}")))?;
    let time = NaiveTime::parse_from_str(&hhmm, "%H:%M")
        .map_err(|_| AppError::Invariant(format!("TimerClock.now_hhmm 返回非法时间：{hhmm}")))?;
    Ok(NaiveDateTime::new(date, time))
}
