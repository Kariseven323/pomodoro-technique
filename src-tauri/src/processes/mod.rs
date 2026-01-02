//! Windows 进程列表与终止逻辑（专注模式核心能力）。

mod enumeration;
pub(crate) mod termination;

pub use enumeration::{list_processes, ProcessInfo};
pub use termination::{kill_names_best_effort, KillSummary};

/// 向前端广播“终止黑名单进程结果”的事件名。
pub const EVENT_KILL_RESULT: &str = "pomodoro://kill_result";
