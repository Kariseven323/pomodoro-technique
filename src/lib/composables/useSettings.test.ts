/** `composables/useSettings.ts` 的单元测试：覆盖保存设置、置顶切换与错误提示。 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { appData, timerSnapshot } from "$lib/stores/appClient";
import type { AppData, Settings } from "$lib/shared/types";

const { updateSettingsMock, setAlwaysOnTopMock } = vi.hoisted(() => ({
  updateSettingsMock: vi.fn(),
  setAlwaysOnTopMock: vi.fn(),
}));

vi.mock("$lib/api/tauri", () => ({
  updateSettings: updateSettingsMock,
  setAlwaysOnTop: setAlwaysOnTopMock,
}));

import { useSettings } from "./useSettings";

/** 构造最小 settings（便于测试 alwaysOnTop 逻辑）。 */
function makeSettings(alwaysOnTop: boolean): Settings {
  return {
    pomodoro: 25,
    shortBreak: 5,
    longBreak: 15,
    longBreakInterval: 4,
    autoContinueEnabled: false,
    autoContinuePomodoros: 4,
    dailyGoal: 0,
    weeklyGoal: 0,
    alwaysOnTop,
    audio: { enabled: true, currentAudioId: "builtin-white-noise", volume: 60, autoPlay: true },
    animation: { enabled: true, comboEnabled: true, intensity: "standard" },
    interruption: { enabled: true, confirmOnInterrupt: true },
  };
}

describe("useSettings", () => {
  beforeEach(() => {
    updateSettingsMock.mockReset();
    setAlwaysOnTopMock.mockReset();
    appData.set(null);
    timerSnapshot.set(null);
  });

  it("saveSettings：无 appData 时返回 false", async () => {
    const showToast = vi.fn();
    const api = useSettings({ showToast });
    await expect(api.saveSettings(makeSettings(false))).resolves.toBe(false);
    expect(updateSettingsMock).not.toHaveBeenCalled();
  });

  it("saveSettings：成功后应同步快照，并在 alwaysOnTop 变更时调用 setAlwaysOnTop", async () => {
    appData.set({ settings: makeSettings(false) } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useSettings({ showToast });

    const next = makeSettings(true);
    const snapshot = { data: { settings: next }, timer: { settings: next } };
    updateSettingsMock.mockResolvedValueOnce(snapshot);
    setAlwaysOnTopMock.mockResolvedValueOnce(true);

    await expect(api.saveSettings(next)).resolves.toBe(true);
    expect(updateSettingsMock).toHaveBeenCalledWith(next);
    expect(setAlwaysOnTopMock).toHaveBeenCalledWith(true);
    expect(showToast).toHaveBeenCalledWith("已保存设置");

    expect(get(appData)?.settings.alwaysOnTop).toBe(true);
  });

  it("saveSettings：alwaysOnTop 未变更时不应调用 setAlwaysOnTop", async () => {
    appData.set({ settings: makeSettings(false) } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useSettings({ showToast });

    const next = makeSettings(false);
    updateSettingsMock.mockResolvedValueOnce({ data: { settings: next }, timer: { settings: next } });
    await expect(api.saveSettings(next)).resolves.toBe(true);
    expect(setAlwaysOnTopMock).not.toHaveBeenCalled();
  });

  it("saveSettings：失败时应提示错误并返回 false", async () => {
    appData.set({ settings: makeSettings(false) } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useSettings({ showToast });

    updateSettingsMock.mockRejectedValueOnce(new Error("boom"));
    await expect(api.saveSettings(makeSettings(false))).resolves.toBe(false);
    expect(showToast).toHaveBeenCalledWith("boom");
  });

  it("toggleAlwaysOnTop：成功时应写回 appData 并提示文案", async () => {
    appData.set({ settings: makeSettings(false) } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useSettings({ showToast });

    setAlwaysOnTopMock.mockResolvedValueOnce(true);
    await api.toggleAlwaysOnTop();

    expect(setAlwaysOnTopMock).toHaveBeenCalledWith(true);
    expect(get(appData)?.settings.alwaysOnTop).toBe(true);
    expect(showToast).toHaveBeenCalledWith("已置顶");
  });

  it("toggleAlwaysOnTop：失败时应提示错误", async () => {
    appData.set({ settings: makeSettings(false) } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useSettings({ showToast });

    setAlwaysOnTopMock.mockRejectedValueOnce(new Error("x"));
    await api.toggleAlwaysOnTop();
    expect(showToast).toHaveBeenCalledWith("x");
  });
});
