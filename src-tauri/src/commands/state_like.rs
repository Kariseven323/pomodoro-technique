//! 命令层可测试状态抽象：用 trait 解耦 `AppState`，便于单元测试 commands/\*.rs。

use crate::app_data::AppData;
use crate::errors::AppResult;
use crate::processes::KillSummary;
use crate::timer::{TimerRuntime, TimerSnapshot};

#[cfg(not(test))]
use crate::state::AppState;

#[cfg(test)]
use std::sync::{atomic::AtomicUsize, atomic::Ordering, Mutex};

/// 命令层依赖的最小状态接口（用于将 `commands/*` 的业务逻辑与 Tauri 运行时解耦）。
pub(crate) trait CommandState {
    /// 获取一份 `AppData` 快照（只读）。
    fn data_snapshot(&self) -> AppData;

    /// 获取计时器快照（只读，包含统计/目标进度等派生字段）。
    fn timer_snapshot(&self) -> TimerSnapshot;

    /// 原子更新：修改数据并持久化（测试实现可忽略持久化）。
    fn update_data(&self, f: impl FnOnce(&mut AppData) -> AppResult<()>) -> AppResult<()>;

    /// 修改计时器运行态（测试实现可直接更新内存）。
    fn update_timer(&self, f: impl FnOnce(&mut TimerRuntime, &AppData) -> AppResult<()>) -> AppResult<()>;

    /// 同时修改 `AppData` 与 `TimerRuntime`（并可控制是否持久化）。
    fn update_data_and_timer<T>(
        &self,
        f: impl FnOnce(&mut AppData, &mut TimerRuntime) -> AppResult<T>,
        persist: bool,
    ) -> AppResult<T>;

    /// 推送当前计时器快照事件给前端（测试实现可记录调用）。
    fn emit_timer_snapshot(&self) -> AppResult<()>;

    /// 推送进程终止结果事件给前端（测试实现可记录 payload）。
    fn emit_kill_result(&self, payload: KillSummary) -> AppResult<()>;

    /// 推送一个“无结构负载”的简单事件给前端（测试实现可记录 event）。
    fn emit_simple_event(&self, event: &str) -> AppResult<()>;
}

#[cfg(not(test))]
impl CommandState for AppState {
    /// 读取一份 `AppData` 的快照（用于命令层返回）。
    fn data_snapshot(&self) -> AppData {
        AppState::data_snapshot(self)
    }

    /// 获取计时器运行态快照（用于命令层返回）。
    fn timer_snapshot(&self) -> TimerSnapshot {
        AppState::timer_snapshot(self)
    }

    /// 原子更新：修改数据并持久化到 store。
    fn update_data(&self, f: impl FnOnce(&mut AppData) -> AppResult<()>) -> AppResult<()> {
        AppState::update_data(self, f)
    }

    /// 修改计时器运行态（不会持久化）。
    fn update_timer(&self, f: impl FnOnce(&mut TimerRuntime, &AppData) -> AppResult<()>) -> AppResult<()> {
        AppState::update_timer(self, f)
    }

    /// 同时修改 `AppData` 与 `TimerRuntime`（需要时可持久化）。
    fn update_data_and_timer<T>(
        &self,
        f: impl FnOnce(&mut AppData, &mut TimerRuntime) -> AppResult<T>,
        persist: bool,
    ) -> AppResult<T> {
        AppState::update_data_and_timer(self, f, persist)
    }

    /// 向前端广播计时器快照事件。
    fn emit_timer_snapshot(&self) -> AppResult<()> {
        AppState::emit_timer_snapshot(self)
    }

    /// 向前端广播进程终止结果事件。
    fn emit_kill_result(&self, payload: KillSummary) -> AppResult<()> {
        AppState::emit_kill_result(self, payload)
    }

    /// 向前端广播简单事件（用于提示刷新）。
    fn emit_simple_event(&self, event: &str) -> AppResult<()> {
        AppState::emit_simple_event(self, event)
    }
}

/// 测试用状态：以内存模拟 `AppState`（不依赖 Tauri runtime / store / AppHandle）。
#[cfg(test)]
pub(crate) struct TestState {
    /// 内存中的持久化数据。
    data: Mutex<AppData>,
    /// 内存中的计时器运行态。
    timer: Mutex<TimerRuntime>,
    /// “计时器快照事件”触发次数。
    emitted_timer_snapshots: AtomicUsize,
    /// 记录所有 kill 结果 payload。
    emitted_kill_results: Mutex<Vec<KillSummary>>,
    /// 记录所有简单事件名。
    emitted_events: Mutex<Vec<String>>,
}

#[cfg(test)]
impl TestState {
    /// 创建一个测试状态（计时器用 `SystemClock` 初始化即可满足命令层测试）。
    pub(crate) fn new(data: AppData) -> Self {
        let clock = crate::timer::SystemClock;
        let timer = TimerRuntime::new(&data.settings, &data.tags, &clock);
        Self {
            data: Mutex::new(data),
            timer: Mutex::new(timer),
            emitted_timer_snapshots: AtomicUsize::new(0),
            emitted_kill_results: Mutex::new(Vec::new()),
            emitted_events: Mutex::new(Vec::new()),
        }
    }

    /// 读取已记录的“计时器快照事件”触发次数。
    pub(crate) fn emitted_timer_snapshot_count(&self) -> usize {
        self.emitted_timer_snapshots.load(Ordering::Relaxed)
    }

    /// 取出已记录的 kill 结果（按调用顺序）。
    pub(crate) fn take_kill_results(&self) -> Vec<KillSummary> {
        std::mem::take(&mut *self.emitted_kill_results.lock().unwrap())
    }

    /// 取出已记录的简单事件名（按调用顺序）。
    pub(crate) fn take_events(&self) -> Vec<String> {
        std::mem::take(&mut *self.emitted_events.lock().unwrap())
    }
}

#[cfg(test)]
impl CommandState for TestState {
    /// 读取一份 `AppData` 的快照（用于断言）。
    fn data_snapshot(&self) -> AppData {
        self.data.lock().unwrap().clone()
    }

    /// 获取计时器快照（用于断言）。
    fn timer_snapshot(&self) -> TimerSnapshot {
        let data = self.data.lock().unwrap();
        let timer = self.timer.lock().unwrap();
        timer.snapshot(&data)
    }

    /// 原子更新：修改数据（测试实现不做持久化）。
    fn update_data(&self, f: impl FnOnce(&mut AppData) -> AppResult<()>) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        f(&mut data)
    }

    /// 修改计时器运行态（测试实现直接更新内存）。
    fn update_timer(&self, f: impl FnOnce(&mut TimerRuntime, &AppData) -> AppResult<()>) -> AppResult<()> {
        let data = self.data.lock().unwrap();
        let mut timer = self.timer.lock().unwrap();
        f(&mut timer, &data)
    }

    /// 同时修改 `AppData` 与 `TimerRuntime`（测试实现忽略 persist 参数）。
    fn update_data_and_timer<T>(
        &self,
        f: impl FnOnce(&mut AppData, &mut TimerRuntime) -> AppResult<T>,
        _persist: bool,
    ) -> AppResult<T> {
        let mut data = self.data.lock().unwrap();
        let mut timer = self.timer.lock().unwrap();
        f(&mut data, &mut timer)
    }

    /// 记录一次“计时器快照事件”触发。
    fn emit_timer_snapshot(&self) -> AppResult<()> {
        self.emitted_timer_snapshots.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 记录一次 kill 结果事件（保存 payload）。
    fn emit_kill_result(&self, payload: KillSummary) -> AppResult<()> {
        self.emitted_kill_results.lock().unwrap().push(payload);
        Ok(())
    }

    /// 记录一个简单事件名（用于断言触发顺序）。
    fn emit_simple_event(&self, event: &str) -> AppResult<()> {
        self.emitted_events.lock().unwrap().push(event.to_string());
        Ok(())
    }
}
