/** `utils/phase.ts` 的单元测试：覆盖阶段展示文本与样式类映射。 */

import { describe, expect, it } from "vitest";
import { phaseAccentClass, phaseLabel } from "./phase";
import type { Phase } from "$lib/shared/types";

/** 构造一个 Phase（便于在测试中集中遍历）。 */
function phases(): Phase[] {
  return ["work", "shortBreak", "longBreak"];
}

describe("phaseLabel", () => {
  it("work/shortBreak/longBreak 映射为中文", () => {
    expect(phaseLabel("work")).toBe("工作");
    expect(phaseLabel("shortBreak")).toBe("短休息");
    expect(phaseLabel("longBreak")).toBe("长休息");
  });

  it("对所有 Phase 都应返回非空字符串", () => {
    for (const p of phases()) {
      expect(phaseLabel(p).length).toBeGreaterThan(0);
    }
  });
});

describe("phaseAccentClass", () => {
  it("work/shortBreak/longBreak 映射为对应 Tailwind class", () => {
    expect(phaseAccentClass("work")).toBe("text-rose-600 dark:text-rose-300");
    expect(phaseAccentClass("shortBreak")).toBe("text-emerald-600 dark:text-emerald-300");
    expect(phaseAccentClass("longBreak")).toBe("text-sky-600 dark:text-sky-300");
  });
});
