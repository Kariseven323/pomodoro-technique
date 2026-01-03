//! 计时器引擎：阶段切换、倒计时、历史记录与通知触发。

pub(crate) mod notification;
mod runtime;
pub(crate) mod stats;
mod validation;

#[cfg(not(test))]
pub use notification::TauriNotifier;
#[cfg(not(test))]
pub use runtime::TickResult;
pub use runtime::{SystemClock, TimerClock, TimerRuntime, TimerSnapshot, WorkCompletedEvent};
pub use stats::compute_today_stats;
pub use validation::validate_settings;

#[cfg(not(test))]
use std::time::Duration;

#[cfg(not(test))]
use tokio::time::sleep;

#[cfg(not(test))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(test))]
use std::sync::Arc;

#[cfg(not(test))]
use tauri::Manager as _;

#[cfg(not(test))]
use crate::state::AppState;

/// 向前端广播计时器状态的事件名。
pub const EVENT_SNAPSHOT: &str = "pomodoro://snapshot";

/// 向前端广播“工作阶段自然完成”的事件名。
pub const EVENT_WORK_COMPLETED: &str = "pomodoro://work_completed";

/// 启动后台 tick 任务：每秒更新计时器、推送事件与刷新托盘。
#[cfg(not(test))]
pub fn spawn_timer_task(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let guard_interval = Duration::from_secs(2);
        let guard_running = Arc::new(AtomicBool::new(false));
        let mut guard_elapsed = guard_interval;
        loop {
            sleep(Duration::from_secs(1)).await;
            let state = app.state::<AppState>();
            let was_running = state.is_running();
            if let Ok(result) = state.tick() {
                if result.work_auto_started {
                    let names = state.blacklist_names_snapshot();
                    let payload = crate::processes::kill_names_best_effort(&names);
                    let _ = state.emit_kill_result(payload);
                }
                if was_running || result.phase_ended {
                    let _ = state.emit_timer_snapshot();
                    let _ = crate::tray::refresh_tray(&state);
                }
            }

            guard_elapsed = guard_elapsed.saturating_add(Duration::from_secs(1));
            if guard_elapsed < guard_interval {
                continue;
            }
            guard_elapsed = Duration::from_secs(0);

            let snapshot = state.timer_snapshot();
            if !(snapshot.is_running
                && snapshot.phase == crate::app_data::Phase::Work
                && snapshot.blacklist_locked)
            {
                continue;
            }

            if guard_running
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }

            let app_handle = app.clone();
            let guard_running = guard_running.clone();
            let names = state.blacklist_names_snapshot();
            tauri::async_runtime::spawn(async move {
                /// 执行一次黑名单守护扫描：在专注期内终止新启动的黑名单进程。
                async fn run_blacklist_guard(app_handle: tauri::AppHandle, names: Vec<String>) {
                    if names.is_empty() {
                        return;
                    }
                    let payload = match tauri::async_runtime::spawn_blocking(move || {
                        crate::processes::termination::kill_names_best_effort_single_snapshot(
                            &names,
                        )
                    })
                    .await
                    {
                        Ok(payload) => payload,
                        Err(_) => return,
                    };
                    if !should_emit_guard_kill_result(&payload) {
                        return;
                    }
                    let state = app_handle.state::<AppState>();
                    let _ = state.emit_kill_result(payload);
                }

                /// 判断守护扫描结果是否需要向前端推送：避免“没有匹配进程”的空结果造成噪声。
                fn should_emit_guard_kill_result(payload: &crate::processes::KillSummary) -> bool {
                    if payload.requires_admin {
                        return true;
                    }
                    payload.items.iter().any(|it| {
                        it.killed > 0 || it.failed > 0 || it.requires_admin || !it.pids.is_empty()
                    })
                }

                run_blacklist_guard(app_handle, names).await;
                guard_running.store(false, Ordering::Release);
            });
        }
    });
}
