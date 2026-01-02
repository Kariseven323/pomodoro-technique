//! 前端事件类型与事件名常量（用于完成动画与里程碑提示）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 前端事件：番茄完成（用于触发完成动画与 Combo）。
pub const EVENT_POMODORO_COMPLETED: &str = "pomodoro-completed";

/// 前端事件：里程碑达成（用于提示 100/500/1000 等累计番茄数）。
pub const EVENT_MILESTONE_REACHED: &str = "milestone-reached";

/// 番茄完成事件负载。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PomodoroCompletedPayload {
    /// 当前 Combo 数。
    pub combo: u32,
    /// 累计完成番茄总数。
    pub total: u64,
    /// 是否达成每日目标（本次完成触发“首次达到”）。
    pub daily_goal_reached: bool,
}

/// 里程碑达成事件负载。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MilestoneReachedPayload {
    /// 里程碑数值（例如 100/500/1000）。
    pub milestone: u64,
}
