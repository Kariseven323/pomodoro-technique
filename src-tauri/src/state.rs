//! 应用后端全局状态（持久化数据 + 计时器运行态 + 托盘句柄）。

use std::sync::Mutex;

use tauri::Emitter as _;

use crate::app_data::{AppData, STORE_KEY};
use crate::errors::{AppError, AppResult};
use crate::timer::{TickResult, TimerRuntime, TimerSnapshot, WorkCompletedEvent};
use crate::tray::TrayHandles;

/// 后端全局状态（通过 `app.manage(...)` 注入 Tauri State）。
pub struct AppState {
    app: tauri::AppHandle,
    store: std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>,
    data: Mutex<AppData>,
    timer: Mutex<TimerRuntime>,
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
    ) -> Self {
        let timer = TimerRuntime::new(&data.settings, &data.tags);
        Self {
            app,
            store,
            data: Mutex::new(data),
            timer: Mutex::new(timer),
            tray: Mutex::new(None),
            window_mode: Mutex::new(WindowModeState::default()),
        }
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
        let mut data = self.data.lock().unwrap();
        f(&mut data)?;
        self.persist_locked(&data)
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
        let result = timer.tick(&mut data, &self.app)?;
        if result.history_changed {
            self.persist_locked(&data)?;
        }
        if let Some(payload) = result.work_completed_event.clone() {
            let _ = self.emit_work_completed(payload);
        }
        Ok(result)
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
