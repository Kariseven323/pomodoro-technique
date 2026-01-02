/** `composables/useTimer.ts` 的单元测试：覆盖开始/暂停切换与错误提示。 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { timerSnapshot } from "$lib/stores/appClient";
import type { TimerSnapshot } from "$lib/shared/types";

const { timerStartMock, timerPauseMock, timerResetMock, timerSkipMock } = vi.hoisted(() => ({
  timerStartMock: vi.fn(),
  timerPauseMock: vi.fn(),
  timerResetMock: vi.fn(),
  timerSkipMock: vi.fn(),
}));

vi.mock("$lib/api/tauri", () => ({
  timerStart: timerStartMock,
  timerPause: timerPauseMock,
  timerReset: timerResetMock,
  timerSkip: timerSkipMock,
}));

import { useTimer } from "./useTimer";

describe("useTimer", () => {
  beforeEach(() => {
    timerStartMock.mockReset();
    timerPauseMock.mockReset();
    timerResetMock.mockReset();
    timerSkipMock.mockReset();
    timerSnapshot.set(null);
  });

  it("toggleStartPause：无快照时应直接返回", async () => {
    const showToast = vi.fn();
    const api = useTimer({ showToast });
    await api.toggleStartPause();
    expect(timerStartMock).not.toHaveBeenCalled();
    expect(timerPauseMock).not.toHaveBeenCalled();
    expect(showToast).not.toHaveBeenCalled();
  });

  it("toggleStartPause：未运行时应调用 timerStart", async () => {
    timerSnapshot.set({ isRunning: false } as Partial<TimerSnapshot> as TimerSnapshot);
    const api = useTimer({ showToast: vi.fn() });
    timerStartMock.mockResolvedValueOnce({});
    await api.toggleStartPause();
    expect(timerStartMock).toHaveBeenCalled();
    expect(timerPauseMock).not.toHaveBeenCalled();
  });

  it("toggleStartPause：运行中应调用 timerPause", async () => {
    timerSnapshot.set({ isRunning: true } as Partial<TimerSnapshot> as TimerSnapshot);
    const api = useTimer({ showToast: vi.fn() });
    timerPauseMock.mockResolvedValueOnce({});
    await api.toggleStartPause();
    expect(timerPauseMock).toHaveBeenCalled();
    expect(timerStartMock).not.toHaveBeenCalled();
  });

  it("toggleStartPause：失败时应提示错误", async () => {
    timerSnapshot.set({ isRunning: false } as Partial<TimerSnapshot> as TimerSnapshot);
    const showToast = vi.fn();
    const api = useTimer({ showToast });
    timerStartMock.mockRejectedValueOnce(new Error("boom"));
    await api.toggleStartPause();
    expect(showToast).toHaveBeenCalledWith("boom");
  });

  it("resetTimer/skipTimer：失败时应提示错误", async () => {
    const showToast = vi.fn();
    const api = useTimer({ showToast });

    timerResetMock.mockRejectedValueOnce(new Error("reset"));
    await api.resetTimer();
    expect(showToast).toHaveBeenCalledWith("reset");

    timerSkipMock.mockRejectedValueOnce(new Error("skip"));
    await api.skipTimer();
    expect(showToast).toHaveBeenCalledWith("skip");
  });
});
