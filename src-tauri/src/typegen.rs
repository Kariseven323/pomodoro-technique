//! TypeScript 类型生成用的“公共重导出”模块（用于 `ts-rs` 的 typegen 工具）。

pub use crate::analysis::{FocusAnalysis, TagEfficiency};
pub use crate::app_data::{
    AppData, BlacklistItem, BlacklistTemplate, DateRange, HistoryDay, HistoryRecord, Phase,
    Settings,
};
pub use crate::commands::types::{AppSnapshot, ExportField, ExportFormat, ExportRequest, StorePaths};
pub use crate::processes::{KillSummary, ProcessInfo};
pub use crate::processes::termination::KillItem;
pub use crate::timer::{TimerSnapshot, WorkCompletedEvent};
pub use crate::timer::stats::{GoalProgress, TagCount, TodayStats, WeekStats};
