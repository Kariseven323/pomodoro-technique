/** `stores/appClient.ts` 的单元测试：覆盖快照写入、事件处理与初始化流程。 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const { listenMock, getAppSnapshotMock } = vi.hoisted(() => ({
  listenMock: vi.fn(),
  getAppSnapshotMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("$lib/api/tauri", () => ({ getAppSnapshot: getAppSnapshotMock }));

import type { AppData, AppSnapshot, TimerSnapshot, WorkCompletedEvent } from "$lib/shared/types";

/** 构造最小的 AppData（仅包含测试所需字段，其他字段用 as 强转）。 */
function makeAppData(partial: Partial<AppData> = {}): AppData {
  return {
    settings: {
      pomodoro: 25,
      shortBreak: 5,
      longBreak: 15,
      longBreakInterval: 4,
      autoContinueEnabled: false,
      autoContinuePomodoros: 4,
      dailyGoal: 0,
      weeklyGoal: 0,
      alwaysOnTop: false,
    },
    blacklist: [],
    blacklistTemplates: [],
    activeTemplateIds: [],
    activeTemplateId: null,
    tags: ["工作"],
    history: [],
    historyDev: [],
    ...partial,
  } as unknown as AppData;
}

/** 构造最小的 TimerSnapshot（仅包含测试所需字段，其他字段用 as 强转）。 */
function makeTimerSnapshot(partial: Partial<TimerSnapshot> = {}): TimerSnapshot {
  return {
    phase: "work",
    remainingSeconds: 25 * 60,
    isRunning: false,
    currentTag: "工作",
    blacklistLocked: false,
    settings: makeAppData().settings,
    todayStats: { total: 0, byTag: [] },
    weekStats: { total: 0, byTag: [] },
    goalProgress: { dailyGoal: 0, dailyCompleted: 0, weeklyGoal: 0, weeklyCompleted: 0 },
    ...partial,
  } as unknown as TimerSnapshot;
}

/** 构造最小的 AppSnapshot。 */
function makeSnapshot(partial: Partial<AppSnapshot> = {}): AppSnapshot {
  return {
    data: makeAppData(),
    timer: makeTimerSnapshot(),
    ...partial,
  } as unknown as AppSnapshot;
}

describe("appClient stores", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("applyAppSnapshot 应写入 appData 与 timerSnapshot", async () => {
    vi.resetModules();
    const mod = await import("./appClient");

    const snapshot = makeSnapshot({
      data: makeAppData({ tags: ["A"] }),
      timer: makeTimerSnapshot({ currentTag: "A" }),
    });

    mod.applyAppSnapshot(snapshot);
    expect(get(mod.appData)?.tags).toEqual(["A"]);
    expect(get(mod.timerSnapshot)?.currentTag).toBe("A");
  });

  it("applyWorkCompletedEvent 应写入 workCompleted 并同步追加到 history", async () => {
    vi.resetModules();
    const mod = await import("./appClient");

    mod.appData.set(
      makeAppData({
        history: [
          {
            date: "2025-01-01",
            records: [{ tag: "A", startTime: "09:00", endTime: null, duration: 25, phase: "work", remark: "" }],
          },
        ],
      }),
    );

    const e: WorkCompletedEvent = {
      date: "2025-01-01",
      recordIndex: 1,
      record: { tag: "B", startTime: "09:30", endTime: null, duration: 25, phase: "work", remark: "" },
    } as unknown as WorkCompletedEvent;

    mod.applyWorkCompletedEvent(e);
    expect(get(mod.workCompleted)).toEqual(e);

    const data = get(mod.appData)!;
    expect(data.history[0]?.records.map((r) => r.tag)).toEqual(["A", "B"]);
  });

  it("initAppClient 应加载快照、注册事件监听，并可通过 handler 更新 store", async () => {
    vi.resetModules();
    listenMock.mockReset();
    getAppSnapshotMock.mockReset();

    const snapshot1 = makeSnapshot({
      data: makeAppData({ tags: ["A"] }),
      timer: makeTimerSnapshot({ currentTag: "A" }),
    });
    const snapshot2 = makeSnapshot({
      data: makeAppData({ tags: ["B"] }),
      timer: makeTimerSnapshot({ currentTag: "B" }),
    });

    getAppSnapshotMock.mockResolvedValueOnce(snapshot1).mockResolvedValueOnce(snapshot2);

    const handlers = new Map<string, (e: { payload: unknown }) => void>();
    listenMock.mockImplementation(async (event: string, handler: (e: { payload: unknown }) => void) => {
      handlers.set(event, handler);
      return () => {};
    });

    const mod = await import("./appClient");
    await mod.initAppClient();

    expect(get(mod.appLoading)).toBe(false);
    expect(get(mod.appError)).toBeNull();
    expect(get(mod.appData)?.tags).toEqual(["A"]);
    expect(get(mod.timerSnapshot)?.currentTag).toBe("A");

    expect(listenMock).toHaveBeenCalledTimes(4);
    expect(handlers.has("pomodoro://snapshot")).toBe(true);
    expect(handlers.has("pomodoro://kill_result")).toBe(true);
    expect(handlers.has("pomodoro://work_completed")).toBe(true);
    expect(handlers.has("pomodoro://history_dev_changed")).toBe(true);

    // snapshot 事件：应更新 timerSnapshot
    handlers.get("pomodoro://snapshot")?.({ payload: makeTimerSnapshot({ currentTag: "X" }) });
    expect(get(mod.timerSnapshot)?.currentTag).toBe("X");

    // work_completed 事件：应更新 workCompleted + history
    handlers.get("pomodoro://work_completed")?.({
      payload: {
        date: "2025-01-02",
        recordIndex: 0,
        record: { tag: "C", startTime: "10:00", endTime: null, duration: 25, phase: "work", remark: "" },
      },
    });
    expect(get(mod.workCompleted)?.date).toBe("2025-01-02");
    expect(get(mod.appData)?.history.some((d) => d.date === "2025-01-02")).toBe(true);

    // history_dev_changed 事件：应刷新时间戳并 best-effort reload snapshot（触发第 2 次 getAppSnapshot）
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2025, 0, 2, 12, 0, 0));
    handlers.get("pomodoro://history_dev_changed")?.({ payload: true });

    // 等待异步 reloadSnapshotBestEffort 完成
    await Promise.resolve();
    await Promise.resolve();

    expect(getAppSnapshotMock).toHaveBeenCalledTimes(2);
    expect(get(mod.appData)?.tags).toEqual(["B"]);
  });

  it("disposeAppClient 应卸载监听并重置 initialized 标记（可重复 init）", async () => {
    vi.resetModules();
    listenMock.mockReset();
    getAppSnapshotMock.mockReset();

    getAppSnapshotMock.mockResolvedValue(makeSnapshot());

    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);

    const mod = await import("./appClient");
    await mod.initAppClient();
    mod.disposeAppClient();
    expect(unlisten).toHaveBeenCalled();

    await mod.initAppClient();
    expect(getAppSnapshotMock).toHaveBeenCalledTimes(2);
  });
});
