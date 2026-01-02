/** `api/tauri.ts` 的单元测试：确保各封装函数调用正确的 Tauri invoke 命令与参数。 */

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import type { BlacklistItem, BlacklistTemplate, DateRange, ExportRequest, Settings } from "$lib/shared/types";
import * as api from "./tauri";

/** 构造一个最小的 DateRange（用于命令参数测试）。 */
function range(): DateRange {
  return { from: "2025-01-01", to: "2025-01-07" };
}

describe("api/tauri wrappers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("getAppSnapshot", async () => {
    const ret = { ok: true };
    invokeMock.mockResolvedValueOnce(ret);
    await expect(api.getAppSnapshot()).resolves.toBe(ret);
    expect(invokeMock).toHaveBeenCalledWith("get_app_snapshot");
  });

  it("getStorePaths / openStoreDir", async () => {
    const paths = { storeDirPath: "x" };
    invokeMock.mockResolvedValueOnce(paths);
    await expect(api.getStorePaths()).resolves.toBe(paths);
    expect(invokeMock).toHaveBeenLastCalledWith("get_store_paths");

    invokeMock.mockResolvedValueOnce(undefined);
    await expect(api.openStoreDir()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenLastCalledWith("open_store_dir");
  });

  it("updateSettings / setGoals", async () => {
    const settings = { pomodoro: 25 } as unknown as Settings;
    const snapshot = { data: { settings }, timer: { settings } };
    invokeMock.mockResolvedValueOnce(snapshot);
    await expect(api.updateSettings(settings)).resolves.toBe(snapshot);
    expect(invokeMock).toHaveBeenLastCalledWith("update_settings", { settings });

    const nextSettings = { pomodoro: 25 } as unknown as Settings;
    invokeMock.mockResolvedValueOnce(nextSettings);
    await expect(api.setGoals(1, 2)).resolves.toBe(nextSettings);
    expect(invokeMock).toHaveBeenLastCalledWith("set_goals", { daily: 1, weekly: 2 });
  });

  it("setCurrentTag / addTag / renameTag / deleteTag", async () => {
    const snapshot = { ok: true };
    invokeMock.mockResolvedValueOnce(snapshot);
    await expect(api.setCurrentTag("学习")).resolves.toBe(snapshot);
    expect(invokeMock).toHaveBeenLastCalledWith("set_current_tag", { tag: "学习" });

    const tags = ["A"];
    invokeMock.mockResolvedValueOnce(tags);
    await expect(api.addTag("A")).resolves.toBe(tags);
    expect(invokeMock).toHaveBeenLastCalledWith("add_tag", { tag: "A" });

    const renamedSnapshot = { ok: "renamed" };
    invokeMock.mockResolvedValueOnce(renamedSnapshot);
    await expect(api.renameTag("旧", "新")).resolves.toBe(renamedSnapshot);
    expect(invokeMock).toHaveBeenLastCalledWith("rename_tag", { from: "旧", to: "新" });

    const deletedSnapshot = { ok: "deleted" };
    invokeMock.mockResolvedValueOnce(deletedSnapshot);
    await expect(api.deleteTag("新")).resolves.toBe(deletedSnapshot);
    expect(invokeMock).toHaveBeenLastCalledWith("delete_tag", { tag: "新" });
  });

  it("setBlacklist", async () => {
    const blacklist: BlacklistItem[] = [{ name: "a.exe", displayName: "A" }];
    invokeMock.mockResolvedValueOnce(blacklist);
    await expect(api.setBlacklist(blacklist)).resolves.toBe(blacklist);
    expect(invokeMock).toHaveBeenLastCalledWith("set_blacklist", { blacklist });
  });

  it("getHistory / setHistoryRemark / getFocusAnalysis", async () => {
    const days = [{ date: "2025-01-01", records: [] }];
    invokeMock.mockResolvedValueOnce(days);
    await expect(api.getHistory(range())).resolves.toBe(days);
    expect(invokeMock).toHaveBeenLastCalledWith("get_history", { range: range() });

    const record = { tag: "A" };
    invokeMock.mockResolvedValueOnce(record);
    await expect(api.setHistoryRemark("2025-01-01", 0, "x")).resolves.toBe(record);
    expect(invokeMock).toHaveBeenLastCalledWith("set_history_remark", {
      date: "2025-01-01",
      recordIndex: 0,
      remark: "x",
    });

    const analysis = { summary: "ok" };
    invokeMock.mockResolvedValueOnce(analysis);
    await expect(api.getFocusAnalysis(range())).resolves.toBe(analysis);
    expect(invokeMock).toHaveBeenLastCalledWith("get_focus_analysis", { range: range() });
  });

  it("templates: getTemplates / saveTemplate / deleteTemplate / applyTemplate", async () => {
    const templates: BlacklistTemplate[] = [];
    invokeMock.mockResolvedValueOnce(templates);
    await expect(api.getTemplates()).resolves.toBe(templates);
    expect(invokeMock).toHaveBeenLastCalledWith("get_templates");

    const tpl: BlacklistTemplate = { id: "x", name: "X", builtin: false, processes: [] };
    invokeMock.mockResolvedValueOnce(tpl);
    await expect(api.saveTemplate(tpl)).resolves.toBe(tpl);
    expect(invokeMock).toHaveBeenLastCalledWith("save_template", { template: tpl });

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.deleteTemplate("x")).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("delete_template", { id: "x" });

    const blacklist: BlacklistItem[] = [];
    invokeMock.mockResolvedValueOnce(blacklist);
    await expect(api.applyTemplate("x")).resolves.toBe(blacklist);
    expect(invokeMock).toHaveBeenLastCalledWith("apply_template", { id: "x" });
  });

  it("window: setAlwaysOnTop / setMiniMode", async () => {
    invokeMock.mockResolvedValueOnce(true);
    await expect(api.setAlwaysOnTop(true)).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("set_always_on_top", { enabled: true });

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.setMiniMode(false)).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("set_mini_mode", { enabled: false });
  });

  it("exportHistory / openLogDir / frontendLog / exitApp", async () => {
    const request = { format: "csv", range: range(), fields: [] } as unknown as ExportRequest;
    invokeMock.mockResolvedValueOnce("x.csv");
    await expect(api.exportHistory(request)).resolves.toBe("x.csv");
    expect(invokeMock).toHaveBeenLastCalledWith("export_history", { request });

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.openLogDir()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("open_log_dir");

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.frontendLog("info", "m")).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("frontend_log", { level: "info", message: "m" });

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.exitApp()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("exit_app");
  });

  it("debug / processes / timer", async () => {
    invokeMock.mockResolvedValueOnce(1);
    await expect(api.debugGenerateHistory(3)).resolves.toBe(1);
    expect(invokeMock).toHaveBeenLastCalledWith("debug_generate_history", { days: 3 });

    invokeMock.mockResolvedValueOnce(true);
    await expect(api.debugClearHistory()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith("debug_clear_history");

    const procs = [{ name: "a.exe" }];
    invokeMock.mockResolvedValueOnce(procs);
    await expect(api.listProcesses()).resolves.toBe(procs);
    expect(invokeMock).toHaveBeenLastCalledWith("list_processes");

    const snap = { isRunning: true };
    invokeMock.mockResolvedValueOnce(snap);
    await expect(api.timerStart()).resolves.toBe(snap);
    expect(invokeMock).toHaveBeenLastCalledWith("timer_start");

    invokeMock.mockResolvedValueOnce(snap);
    await expect(api.timerPause()).resolves.toBe(snap);
    expect(invokeMock).toHaveBeenLastCalledWith("timer_pause");

    invokeMock.mockResolvedValueOnce(snap);
    await expect(api.timerReset()).resolves.toBe(snap);
    expect(invokeMock).toHaveBeenLastCalledWith("timer_reset");

    invokeMock.mockResolvedValueOnce(snap);
    await expect(api.timerSkip()).resolves.toBe(snap);
    expect(invokeMock).toHaveBeenLastCalledWith("timer_skip");

    invokeMock.mockResolvedValueOnce(undefined);
    await expect(api.restartAsAdmin()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenLastCalledWith("restart_as_admin");
  });
});
