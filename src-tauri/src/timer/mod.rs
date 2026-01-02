//! 计时器引擎：阶段切换、倒计时、历史记录与通知触发。

pub(crate) mod notification;
mod runtime;
pub(crate) mod stats;
mod validation;

pub use notification::TauriNotifier;
pub use runtime::{SystemClock, TickResult, TimerClock, TimerRuntime, TimerSnapshot, WorkCompletedEvent};
pub use stats::compute_today_stats;
pub use validation::validate_settings;

use std::time::Duration;

use tokio::time::sleep;

use tauri::Manager as _;

use crate::state::AppState;

/// 向前端广播计时器状态的事件名。
pub const EVENT_SNAPSHOT: &str = "pomodoro://snapshot";

/// 向前端广播“工作阶段自然完成”的事件名。
pub const EVENT_WORK_COMPLETED: &str = "pomodoro://work_completed";

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
