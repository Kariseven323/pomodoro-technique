//! 应用后端全局状态（持久化数据 + 计时器运行态 + 托盘句柄）。

use std::sync::Mutex;

use tauri::Emitter as _;

use crate::app_data::{AppData, STORE_KEY};
use crate::errors::{AppError, AppResult};
use crate::timer::{TickResult, TimerClock, TimerRuntime, TimerSnapshot, WorkCompletedEvent};
use crate::tray::TrayHandles;

/// 后端全局状态（通过 `app.manage(...)` 注入 Tauri State）。
pub struct AppState {
    app: tauri::AppHandle,
    store: std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>,
    data: Mutex<AppData>,
    timer: Mutex<TimerRuntime>,
    audio: crate::audio::AudioController,
    combo: Mutex<crate::combo::ComboRuntime>,
    tray: Mutex<Option<TrayHandles>>,
    window_mode: Mutex<WindowModeState>,
}

/// 窗口模式运行态（用于迷你模式恢复窗口大小/位置）。
#[derive(Debug, Clone, Default)]
pub struct WindowModeState {
    /// 是否处于迷你模式。
    pub mini_mode: bool,
    /// 进入迷你模式前的窗口尺寸（逻辑像素）。
    pub prev_size: Option<(u32, u32)>,
    /// 进入迷你模式前的窗口位置（逻辑像素）。
    pub prev_position: Option<(i32, i32)>,
}

impl AppState {
    /// 创建应用状态并初始化计时器为“工作阶段 + 默认时长”。
    pub fn new(
        app: tauri::AppHandle,
        store: std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>,
        data: AppData,
    ) -> AppResult<Self> {
        let clock = crate::timer::SystemClock;
        let timer = TimerRuntime::new(&data.settings, &data.tags, &clock);
        let audio_dir = crate::app_paths::app_audio_dir(&app)?;
        let audio = crate::audio::AudioController::new(audio_dir)?;
        audio.update_custom_audios(data.custom_audios.clone())?;
        Ok(Self {
            app,
            store,
            data: Mutex::new(data),
            timer: Mutex::new(timer),
            audio,
            combo: Mutex::new(crate::combo::ComboRuntime::new()),
            tray: Mutex::new(None),
            window_mode: Mutex::new(WindowModeState::default()),
        })
    }

    /// 读取一份 `AppData` 的快照（用于向前端返回）。
    pub fn data_snapshot(&self) -> AppData {
        self.data.lock().unwrap().clone()
    }

    /// 获取计时器运行态快照（用于前端渲染/托盘刷新）。
    pub fn timer_snapshot(&self) -> TimerSnapshot {
        let data = self.data.lock().unwrap();
        let timer = self.timer.lock().unwrap();
        timer.snapshot(&data)
    }

    /// 设置托盘句柄，供后续更新图标/菜单。
    pub fn set_tray(&self, tray: TrayHandles) {
        *self.tray.lock().unwrap() = Some(tray);
    }

    /// 获取托盘句柄（若未创建则为 `None`）。
    pub fn tray(&self) -> Option<TrayHandles> {
        self.tray.lock().unwrap().clone()
    }

    /// 获取窗口模式状态快照（用于命令内部决策）。
    pub fn window_mode_snapshot(&self) -> WindowModeState {
        self.window_mode.lock().unwrap().clone()
    }

    /// 原子更新窗口模式状态（不持久化）。
    pub fn update_window_mode(
        &self,
        f: impl FnOnce(&mut WindowModeState) -> AppResult<()>,
    ) -> AppResult<()> {
        let mut mode = self.window_mode.lock().unwrap();
        f(&mut mode)
    }

    /// 原子更新：修改数据并持久化到 store。
    pub fn update_data(&self, f: impl FnOnce(&mut AppData) -> AppResult<()>) -> AppResult<()> {
        self.update_data_with(f)?;
        Ok(())
    }

    /// 原子更新：修改数据并持久化到 store，同时返回一个计算结果。
    pub fn update_data_with<T>(
        &self,
        f: impl FnOnce(&mut AppData) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut data = self.data.lock().unwrap();
        let out = f(&mut data)?;
        self.persist_locked(&data)?;
        Ok(out)
    }

    /// 修改计时器运行态（不会持久化）。
    pub fn update_timer(
        &self,
        f: impl FnOnce(&mut TimerRuntime, &AppData) -> AppResult<()>,
    ) -> AppResult<()> {
        let data = self.data.lock().unwrap();
        let mut timer = self.timer.lock().unwrap();
        f(&mut timer, &data)
    }

    /// 同时修改 `AppData` 与 `TimerRuntime`（需要时可持久化）。
    pub fn update_data_and_timer<T>(
        &self,
        f: impl FnOnce(&mut AppData, &mut TimerRuntime) -> AppResult<T>,
        persist: bool,
    ) -> AppResult<T> {
        let mut data = self.data.lock().unwrap();
        let mut timer = self.timer.lock().unwrap();
        let out = f(&mut data, &mut timer)?;
        if persist {
            self.persist_locked(&data)?;
        }
        Ok(out)
    }

    /// 获取音频控制器引用（命令层用于触发播放/暂停/音量等操作）。
    pub fn audio_controller(&self) -> &crate::audio::AudioController {
        &self.audio
    }

    /// 获取音频目录路径（用于导入/删除/列表等命令复用）。
    pub fn audio_dir(&self) -> &std::path::Path {
        self.audio.audio_dir()
    }

    /// 将音频状态与当前计时器状态同步（自动播放/暂停 + 淡出）。
    pub fn sync_audio_with_timer(&self) -> AppResult<()> {
        let data = self.data.lock().unwrap();
        let timer = self.timer.lock().unwrap();
        self.audio
            .update_custom_audios(data.custom_audios.clone())
            .ok();
        self.audio.sync_timer(
            data.settings.audio.clone(),
            timer.phase,
            timer.is_running,
            timer.remaining_seconds,
        )?;
        Ok(())
    }

    /// 推送当前计时器快照事件给前端。
    pub fn emit_timer_snapshot(&self) -> AppResult<()> {
        self.app
            .emit(crate::timer::EVENT_SNAPSHOT, self.timer_snapshot())?;
        Ok(())
    }

    /// 推送进程终止结果事件给前端。
    pub fn emit_kill_result(&self, payload: crate::processes::KillSummary) -> AppResult<()> {
        self.app
            .emit(crate::processes::EVENT_KILL_RESULT, payload)?;
        Ok(())
    }

    /// 推送“工作阶段完成并写入历史”的事件给前端。
    pub fn emit_work_completed(&self, payload: WorkCompletedEvent) -> AppResult<()> {
        self.app.emit(crate::timer::EVENT_WORK_COMPLETED, payload)?;
        Ok(())
    }

    /// 推送一个“无结构负载”的简单事件给前端（用于提示 UI 刷新）。
    pub fn emit_simple_event(&self, event: &str) -> AppResult<()> {
        self.app.emit(event, true)?;
        Ok(())
    }

    /// 执行一次 tick：若计时器运行中则可能写入历史并持久化。
    pub fn tick(&self) -> AppResult<TickResult> {
        let mut data = self.data.lock().unwrap();
        let mut timer = self.timer.lock().unwrap();
        let clock = crate::timer::SystemClock;
        let notifier = crate::timer::TauriNotifier::new(&self.app);
        let result = timer.tick(&mut data, &clock, &notifier)?;
        let mut persist_needed = result.history_changed;

        if result.work_auto_started {
            self.combo.lock().unwrap().on_work_started(&clock)?;
        }

        if let Some(payload) = result.work_completed_event.clone() {
            let today = clock.today_date();
            let today_completed_after = crate::timer::compute_today_stats(&data, &today).total;
            let daily_goal_reached =
                data.settings.daily_goal > 0 && today_completed_after == data.settings.daily_goal;

            data.total_pomodoros = data.total_pomodoros.saturating_add(1);
            persist_needed = true;

            let expected_break = timer.phase;
            let settings_snapshot = data.settings.clone();
            let combo = self.combo.lock().unwrap().on_work_completed(
                &mut data,
                &clock,
                expected_break,
                &settings_snapshot,
            )?;

            let _ = self.emit_work_completed(payload);
            let _ = self.emit_pomodoro_completed(combo, data.total_pomodoros, daily_goal_reached);

            let _ = self.emit_milestone_if_needed(data.total_pomodoros);
        }

        // PRD v4：自动播放/淡出需要每秒同步一次。
        self.audio
            .update_custom_audios(data.custom_audios.clone())
            .ok();
        let _ = self.audio.sync_timer(
            data.settings.audio.clone(),
            timer.phase,
            timer.is_running,
            timer.remaining_seconds,
        );

        if persist_needed {
            self.persist_locked(&data)?;
        }
        Ok(result)
    }

    /// 推送“番茄完成”事件给前端（用于完成动画与 Combo）。
    pub fn emit_pomodoro_completed(
        &self,
        combo: u32,
        total: u64,
        daily_goal_reached: bool,
    ) -> AppResult<()> {
        self.app.emit(
            crate::events::EVENT_POMODORO_COMPLETED,
            crate::events::PomodoroCompletedPayload {
                combo,
                total,
                daily_goal_reached,
            },
        )?;
        Ok(())
    }

    /// 若达到里程碑则推送事件给前端（100/500/1000）。
    pub fn emit_milestone_if_needed(&self, total: u64) -> AppResult<()> {
        let milestones = [100u64, 500u64, 1000u64];
        if milestones.contains(&total) {
            self.app.emit(
                crate::events::EVENT_MILESTONE_REACHED,
                crate::events::MilestoneReachedPayload { milestone: total },
            )?;
        }
        Ok(())
    }

    /// 在“工作阶段首次开始”时更新 Combo 判定窗口（用于下一次完成时累加）。
    pub fn on_work_started_for_combo(&self) -> AppResult<()> {
        let clock = crate::timer::SystemClock;
        let mut combo = self.combo.lock().unwrap();
        combo.on_work_started(&clock)?;
        Ok(())
    }

    /// 在“工作阶段中断”时重置 Combo（PRD v4）。
    pub fn on_interrupted_for_combo(&self) -> AppResult<()> {
        self.update_data_and_timer(
            |data, _timer| {
                self.combo.lock().unwrap().on_interrupted(data);
                Ok(())
            },
            true,
        )?;
        Ok(())
    }

    /// 在持有 `AppData` 可变借用时重置 Combo（用于中断记录等需要原子更新的场景）。
    pub(crate) fn reset_combo_locked(&self, data: &mut AppData) {
        self.combo.lock().unwrap().on_interrupted(data);
    }

    /// 退出前记录“quit 中断”（仅在工作阶段已开始且启用记录中断时写入）。
    pub fn record_quit_interruption_before_exit(&self) -> AppResult<()> {
        let wrote = self.update_data_and_timer(
            |data, timer_runtime| {
                if !data.settings.interruption.enabled {
                    return Ok(false);
                }
                if timer_runtime.phase != crate::app_data::Phase::Work
                    || !timer_runtime.is_work_started()
                {
                    return Ok(false);
                }

                let timestamp = chrono::Utc::now().to_rfc3339();
                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                let record = crate::app_data::InterruptionRecord {
                    timestamp,
                    remaining_seconds: timer_runtime.remaining_seconds,
                    focused_seconds: timer_runtime.focused_seconds(&data.settings),
                    reason: String::new(),
                    r#type: crate::app_data::InterruptionType::Quit,
                    tag: timer_runtime.current_tag.clone(),
                };

                // 写入中断记录。
                {
                    let list = &mut data.interruptions;
                    if let Some(idx) = list.iter().position(|d| d.date == date) {
                        list[idx].records.push(record);
                    } else {
                        list.push(crate::app_data::InterruptionDay {
                            date,
                            records: vec![record],
                        });
                    }
                }

                // PRD v4：退出中断也会打断 streak。
                self.combo.lock().unwrap().on_interrupted(data);
                Ok(true)
            },
            false,
        )?;

        if wrote {
            let data = self.data.lock().unwrap();
            self.persist_locked(&data)?;
        }
        Ok(())
    }

    /// 持久化 `AppData` 到 store（要求调用方已持有锁，避免重复锁）。
    fn persist_locked(&self, data: &AppData) -> AppResult<()> {
        self.store.set(
            STORE_KEY,
            serde_json::to_value(data).map_err(AppError::from)?,
        );
        self.store.save()?;
        tracing::debug!(target: "storage", "数据已持久化到 store");
        Ok(())
    }

    /// 判断计时器是否运行中（给托盘菜单逻辑使用）。
    pub fn is_running(&self) -> bool {
        self.timer.lock().unwrap().is_running
    }
}
