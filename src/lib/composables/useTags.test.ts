/** `composables/useTags.ts` 的单元测试：覆盖标签选择、新增与错误提示。 */

import { beforeEach, describe, expect, it, vi } from "vitest";

const { setCurrentTagMock, applyAppSnapshotMock } = vi.hoisted(() => ({
  setCurrentTagMock: vi.fn(),
  applyAppSnapshotMock: vi.fn(),
}));

vi.mock("$lib/api/tauri", () => ({ setCurrentTag: setCurrentTagMock }));
vi.mock("$lib/stores/appClient", () => ({ applyAppSnapshot: applyAppSnapshotMock }));

import { useTags } from "./useTags";

describe("useTags", () => {
  beforeEach(() => {
    setCurrentTagMock.mockReset();
    applyAppSnapshotMock.mockReset();
  });

  it("onTagSelectChange：空白标签应直接返回", async () => {
    const api = useTags({ showToast: vi.fn() });
    await api.onTagSelectChange("   ");
    expect(setCurrentTagMock).not.toHaveBeenCalled();
  });

  it("onTagSelectChange：应 trim 并调用 setCurrentTag，同步 applyAppSnapshot", async () => {
    const api = useTags({ showToast: vi.fn() });
    const snapshot = { ok: true };
    setCurrentTagMock.mockResolvedValueOnce(snapshot);
    await api.onTagSelectChange(" 学习 ");
    expect(setCurrentTagMock).toHaveBeenCalledWith("学习");
    expect(applyAppSnapshotMock).toHaveBeenCalledWith(snapshot);
  });

  it("addAndSelectTag：成功应提示“已添加标签”", async () => {
    const showToast = vi.fn();
    const api = useTags({ showToast });
    setCurrentTagMock.mockResolvedValueOnce({ ok: true });
    await api.addAndSelectTag("新标签");
    expect(showToast).toHaveBeenCalledWith("已添加标签");
  });

  it("失败时应提示错误", async () => {
    const showToast = vi.fn();
    const api = useTags({ showToast });
    setCurrentTagMock.mockRejectedValueOnce(new Error("boom"));
    await api.onTagSelectChange("A");
    expect(showToast).toHaveBeenCalledWith("boom");
  });
});
