/** 应用前端全局状态：集中维护 `AppData`/`TimerSnapshot` 与 Tauri 事件监听。 */

import { isTauri } from "@tauri-apps/api/core";
import { listen, type Event as TauriEvent, type UnlistenFn } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import { frontendLog, getAppSnapshot } from "$lib/api/tauri";
import { miniMode } from "$lib/stores/uiState";
import type {
  AppData,
  AppSnapshot,
  CustomAudio,
  HistoryDay,
  HistoryRecord,
  KillSummary,
  MilestoneReachedPayload,
  PomodoroCompletedPayload,
  TimerSnapshot,
  WorkCompletedEvent,
} from "$lib/shared/types";

/** 全局：持久化数据快照（settings/blacklist/tags/history）。 */
export const appData = writable<AppData | null>(null);

/** 全局：计时器状态快照（实时更新）。 */
export const timerSnapshot = writable<TimerSnapshot | null>(null);

/** 全局：最近一次终止进程结果（用于提示提权/失败原因）。 */
export const killSummary = writable<KillSummary | null>(null);

/** 全局：最近一次“工作阶段完成”事件（用于弹出备注填写）。 */
export const workCompleted = writable<WorkCompletedEvent | null>(null);

/** 全局：最近一次“番茄完成”事件（用于完成动画）。 */
export const pomodoroCompleted = writable<PomodoroCompletedPayload | null>(null);

/** 全局：最近一次“里程碑达成”事件（用于提示与庆祝）。 */
export const milestoneReached = writable<MilestoneReachedPayload | null>(null);

/** 全局：初始化加载状态。 */
export const appLoading = writable<boolean>(true);

/** 全局：初始化错误信息。 */
export const appError = writable<string | null>(null);

/** 全局：调试历史数据变更时间戳（用于历史页面自动刷新）。 */
export const historyDevChangedAt = writable<number>(0);

/** 全局：自定义音频列表变更时间戳（用于主界面下拉框刷新）。 */
export const audioLibraryChangedAt = writable<number>(0);

let initialized = false;
let unlistenFns: UnlistenFn[] = [];

/** 判断当前是否处于 Tauri 宿主环境（避免浏览器环境下 invoke/listen 永久 pending）。 */
function isTauriRuntimeSafe(): boolean {
  if (import.meta.env.VITEST) return true;
  try {
    return isTauri();
  } catch {
    return false;
  }
}

/** 将一个 Promise 包装为“超时失败”，避免初始化阶段永久卡住。 */
async function withTimeout<T>(p: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  let timeoutId: number | null = null;
  try {
    return await Promise.race([
      p,
      new Promise<T>((_, reject) => {
        timeoutId = window.setTimeout(() => reject(new Error(message)), timeoutMs);
      }),
    ]);
  } finally {
    if (timeoutId !== null) window.clearTimeout(timeoutId);
  }
}

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

/** 将番茄完成事件写入 store，并同步更新 `AppData` 中的 combo/累计番茄数。 */
export function applyPomodoroCompletedEvent(e: PomodoroCompletedPayload): void {
  pomodoroCompleted.set(e);
  appData.update((data): AppData | null => {
    if (!data) return data;
    return { ...data, currentCombo: e.combo, totalPomodoros: e.total };
  });
}

/** 初始化：加载快照并注册事件监听（全局只执行一次）。 */
export async function initAppClient(): Promise<void> {
  if (initialized) return;
  initialized = true;
  appLoading.set(true);
  appError.set(null);
  let ok = false;
  try {
    if (!isTauriRuntimeSafe()) {
      throw new Error("当前未运行在桌面端（Tauri）环境，请使用 `bun run tauri dev` 或桌面应用启动。");
    }

    void withTimeout(frontendLog("info", "[frontend] initAppClient: start"), 800, "frontend_log timeout").catch(
      () => {},
    );
    const snapshot = await withTimeout(getAppSnapshot(), 15000, "获取后端快照超时，请重启应用后重试。");
    applyAppSnapshot(snapshot);
    void withTimeout(
      frontendLog("info", "[frontend] initAppClient: snapshot loaded"),
      800,
      "frontend_log timeout",
    ).catch(() => {});
    ok = true;
  } catch (e) {
    appError.set(e instanceof Error ? e.message : String(e));
    void withTimeout(
      frontendLog("error", `[frontend] initAppClient failed: ${String(e)}`),
      800,
      "frontend_log timeout",
    ).catch(() => {});
  } finally {
    appLoading.set(false);
    if (!ok) initialized = false;
  }

  if (ok) {
    try {
      await withTimeout(registerListeners(), 5000, "注册后端事件监听超时（listen 未返回）。");
    } catch (e) {
      void withTimeout(
        frontendLog("warn", `[frontend] registerListeners failed: ${e instanceof Error ? e.message : String(e)}`),
        800,
        "frontend_log timeout",
      ).catch(() => {});
    }
  }
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
  if (!isTauriRuntimeSafe()) return;

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

  /** 处理后端推送的“番茄完成”事件。 */
  function onPomodoroCompletedEvent(e: TauriEvent<PomodoroCompletedPayload>): void {
    applyPomodoroCompletedEvent(e.payload);
  }

  /** 处理后端推送的“里程碑达成”事件。 */
  function onMilestoneReachedEvent(e: TauriEvent<MilestoneReachedPayload>): void {
    milestoneReached.set(e.payload);
  }

  /** 处理后端推送的“音频库变更”事件：更新 `AppData.customAudios` 并通知依赖组件刷新。 */
  function onAudioLibraryChangedEvent(e: TauriEvent<CustomAudio[]>): void {
    audioLibraryChangedAt.set(Date.now());
    appData.update((data): AppData | null => {
      if (!data) return data;
      return { ...data, customAudios: e.payload };
    });
  }

  /** 处理后端推送的“迷你模式变更”事件：同步到前端 UI 状态。 */
  function onMiniModeChangedEvent(e: TauriEvent<boolean>): void {
    miniMode.set(Boolean(e.payload));
  }

  unlistenFns.push(await listen<TimerSnapshot>("pomodoro://snapshot", onTimerSnapshotEvent));
  unlistenFns.push(await listen<KillSummary>("pomodoro://kill_result", onKillResultEvent));
  unlistenFns.push(await listen<WorkCompletedEvent>("pomodoro://work_completed", onWorkCompletedEvent));
  unlistenFns.push(await listen<boolean>("pomodoro://history_dev_changed", onHistoryDevChangedEvent));
  unlistenFns.push(await listen<PomodoroCompletedPayload>("pomodoro-completed", onPomodoroCompletedEvent));
  unlistenFns.push(await listen<MilestoneReachedPayload>("milestone-reached", onMilestoneReachedEvent));
  unlistenFns.push(await listen<CustomAudio[]>("pomodoro://audio_library_changed", onAudioLibraryChangedEvent));
  unlistenFns.push(await listen<boolean>("pomodoro://mini_mode_changed", onMiniModeChangedEvent));
}
