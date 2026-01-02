/** 应用前端全局状态：集中维护 `AppData`/`TimerSnapshot` 与 Tauri 事件监听。 */

import { listen, type Event as TauriEvent, type UnlistenFn } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import { getAppSnapshot } from "$lib/api/tauri";
import type { AppData, AppSnapshot, HistoryDay, HistoryRecord, KillSummary, TimerSnapshot, WorkCompletedEvent } from "$lib/shared/types";

/** 全局：持久化数据快照（settings/blacklist/tags/history）。 */
export const appData = writable<AppData | null>(null);

/** 全局：计时器状态快照（实时更新）。 */
export const timerSnapshot = writable<TimerSnapshot | null>(null);

/** 全局：最近一次终止进程结果（用于提示提权/失败原因）。 */
export const killSummary = writable<KillSummary | null>(null);

/** 全局：最近一次“工作阶段完成”事件（用于弹出备注填写）。 */
export const workCompleted = writable<WorkCompletedEvent | null>(null);

/** 全局：初始化加载状态。 */
export const appLoading = writable<boolean>(true);

/** 全局：初始化错误信息。 */
export const appError = writable<string | null>(null);

/** 全局：调试历史数据变更时间戳（用于历史页面自动刷新）。 */
export const historyDevChangedAt = writable<number>(0);

let initialized = false;
let unlistenFns: UnlistenFn[] = [];

/** 将后端返回的 `AppSnapshot` 写入全局 store。 */
export function applyAppSnapshot(snapshot: AppSnapshot): void {
  appData.set(snapshot.data);
  timerSnapshot.set(snapshot.timer);
}

/** 将工作完成事件写入 store，并同步追加到 `appData.history`（用于 UI 即时展示）。 */
export function applyWorkCompletedEvent(e: WorkCompletedEvent): void {
  workCompleted.set(e);
  appData.update((data): AppData | null => {
    if (!data) return data;
    const nextHistory: HistoryDay[] = data.history.map((d) => ({ ...d, records: d.records.map((r) => ({ ...r })) }));
    const idx = nextHistory.findIndex((d) => d.date === e.date);
    if (idx >= 0) {
      const day = nextHistory[idx];
      const records: HistoryRecord[] = [...day.records];
      records.splice(Math.min(e.recordIndex, records.length), 0, e.record);
      nextHistory[idx] = { ...day, records };
    } else {
      nextHistory.push({ date: e.date, records: [e.record] });
    }
    return { ...data, history: nextHistory };
  });
}

/** 初始化：加载快照并注册事件监听（全局只执行一次）。 */
export async function initAppClient(): Promise<void> {
  if (initialized) return;
  initialized = true;
  appLoading.set(true);
  appError.set(null);
  try {
    const snapshot = await getAppSnapshot();
    applyAppSnapshot(snapshot);
  } catch (e) {
    appError.set(e instanceof Error ? e.message : String(e));
  } finally {
    appLoading.set(false);
  }

  await registerListeners();
}

/** 释放事件监听（一般不需要；仅用于热重载/测试场景）。 */
export function disposeAppClient(): void {
  for (const fn of unlistenFns) {
    try {
      fn();
    } catch {
      // 忽略卸载时错误
    }
  }
  unlistenFns = [];
  initialized = false;
}

/** 重新拉取后端快照并覆盖全局 store（用于调试数据变更后的同步）。 */
async function reloadSnapshotBestEffort(): Promise<void> {
  try {
    const snapshot = await getAppSnapshot();
    applyAppSnapshot(snapshot);
  } catch {
    // 忽略刷新失败：由调用方决定是否提示
  }
}

/** 注册 Tauri 后端事件监听。 */
async function registerListeners(): Promise<void> {
  if (unlistenFns.length > 0) return;

  /** 处理后端推送的计时器快照事件。 */
  function onTimerSnapshotEvent(e: TauriEvent<TimerSnapshot>): void {
    timerSnapshot.set(e.payload);
  }

  /** 处理后端推送的终止进程结果事件。 */
  function onKillResultEvent(e: TauriEvent<KillSummary>): void {
    killSummary.set(e.payload);
  }

  /** 处理后端推送的“工作阶段完成”事件。 */
  function onWorkCompletedEvent(e: TauriEvent<WorkCompletedEvent>): void {
    applyWorkCompletedEvent(e.payload);
  }

  /** 处理后端推送的“调试历史数据变更”事件。 */
  function onHistoryDevChangedEvent(): void {
    historyDevChangedAt.set(Date.now());
    void reloadSnapshotBestEffort();
  }

  unlistenFns.push(await listen<TimerSnapshot>("pomodoro://snapshot", onTimerSnapshotEvent));
  unlistenFns.push(await listen<KillSummary>("pomodoro://kill_result", onKillResultEvent));
  unlistenFns.push(await listen<WorkCompletedEvent>("pomodoro://work_completed", onWorkCompletedEvent));
  unlistenFns.push(await listen<boolean>("pomodoro://history_dev_changed", onHistoryDevChangedEvent));
}
