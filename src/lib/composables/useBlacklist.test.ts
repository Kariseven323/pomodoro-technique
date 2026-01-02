/** `composables/useBlacklist.ts` 的单元测试：覆盖保存黑名单、模板变更同步与错误提示。 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { appData } from "$lib/stores/appClient";
import type { AppData } from "$lib/shared/types";

const { setBlacklistMock } = vi.hoisted(() => ({ setBlacklistMock: vi.fn() }));
vi.mock("$lib/api/tauri", () => ({ setBlacklist: setBlacklistMock }));

import { useBlacklist } from "./useBlacklist";

describe("useBlacklist", () => {
  beforeEach(() => {
    setBlacklistMock.mockReset();
    appData.set(null);
  });

  it("saveBlacklist：无 appData 时返回 false", async () => {
    const api = useBlacklist({ showToast: vi.fn() });
    await expect(api.saveBlacklist([])).resolves.toBe(false);
    expect(setBlacklistMock).not.toHaveBeenCalled();
  });

  it("saveBlacklist：成功应写回 appData.blacklist 并提示", async () => {
    appData.set({
      blacklist: [],
      blacklistTemplates: [],
      activeTemplateIds: [],
      activeTemplateId: null,
    } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useBlacklist({ showToast });

    const next = [{ name: "a.exe", displayName: "A" }];
    setBlacklistMock.mockResolvedValueOnce(next);

    await expect(api.saveBlacklist(next)).resolves.toBe(true);
    expect(get(appData)?.blacklist).toEqual(next);
    expect(showToast).toHaveBeenCalledWith("黑名单已更新");
  });

  it("saveBlacklist：失败应提示错误并返回 false", async () => {
    appData.set({ blacklist: [] } as Partial<AppData> as AppData);
    const showToast = vi.fn();
    const api = useBlacklist({ showToast });

    setBlacklistMock.mockRejectedValueOnce(new Error("boom"));
    await expect(api.saveBlacklist([])).resolves.toBe(false);
    expect(showToast).toHaveBeenCalledWith("boom");
  });

  it("applyTemplatesChange：应写回模板列表、activeTemplateIds/Id 与 blacklist", () => {
    appData.set({
      blacklist: [],
      blacklistTemplates: [],
      activeTemplateIds: [],
      activeTemplateId: null,
    } as Partial<AppData> as AppData);
    const api = useBlacklist({ showToast: vi.fn() });

    api.applyTemplatesChange({
      templates: [{ id: "t1", name: "T1", builtin: true, processes: [] }],
      activeTemplateIds: ["t1"],
      blacklist: [{ name: "a.exe", displayName: "A" }],
    });

    const data = get(appData)!;
    expect(data.blacklistTemplates.length).toBe(1);
    expect(data.activeTemplateIds).toEqual(["t1"]);
    expect(data.activeTemplateId).toBe("t1");
    expect(data.blacklist).toEqual([{ name: "a.exe", displayName: "A" }]);
  });
});
