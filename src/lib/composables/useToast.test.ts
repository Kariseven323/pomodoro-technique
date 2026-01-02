/** `composables/useToast.ts` 的单元测试：覆盖展示、清理与自动隐藏逻辑。 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { useToast } from "./useToast";

describe("useToast", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("showToast 应设置文本，并在超时后自动清空", () => {
    vi.useFakeTimers();
    const { toast, showToast } = useToast({ autoHideMs: 100 });

    showToast("hi");
    expect(get(toast)).toBe("hi");

    vi.advanceTimersByTime(99);
    expect(get(toast)).toBe("hi");

    vi.advanceTimersByTime(1);
    expect(get(toast)).toBeNull();
  });

  it("clearToast 应立即清空，并取消自动隐藏定时器", () => {
    vi.useFakeTimers();
    const { toast, showToast, clearToast } = useToast({ autoHideMs: 100 });

    showToast("hi");
    clearToast();
    expect(get(toast)).toBeNull();

    vi.advanceTimersByTime(100);
    expect(get(toast)).toBeNull();
  });

  it("多次 showToast 应覆盖并重置定时器", () => {
    vi.useFakeTimers();
    const { toast, showToast } = useToast({ autoHideMs: 100 });

    showToast("a");
    vi.advanceTimersByTime(60);
    showToast("b");
    expect(get(toast)).toBe("b");

    vi.advanceTimersByTime(60);
    expect(get(toast)).toBe("b");
    vi.advanceTimersByTime(40);
    expect(get(toast)).toBeNull();
  });
});
