/** `utils/date.ts` 的单元测试：覆盖日期解析、格式化与范围计算。 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { addDays, endOfMonthYmd, formatYmd, parseYmd, startOfMonthYmd, startOfWeekYmd, todayYmd } from "./date";

describe("date utils", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("parseYmd + formatYmd 应互为逆操作（合法日期）", () => {
    expect(formatYmd(parseYmd("2025-01-01"))).toBe("2025-01-01");
    expect(formatYmd(parseYmd("2025-12-31"))).toBe("2025-12-31");
  });

  it("formatYmd 输出应为 YYYY-MM-DD", () => {
    const out = formatYmd(new Date(2025, 0, 2, 12, 0, 0));
    expect(out).toBe("2025-01-02");
  });

  it("todayYmd 应返回当天（mock Date）", () => {
    vi.useFakeTimers();
    // 使用本地中午，避免时区导致跨日。
    vi.setSystemTime(new Date(2025, 0, 2, 12, 0, 0));
    expect(todayYmd()).toBe("2025-01-02");
  });

  it("addDays 支持正负天数与跨月/跨年", () => {
    expect(addDays("2025-01-01", 1)).toBe("2025-01-02");
    expect(addDays("2025-01-01", -1)).toBe("2024-12-31");
    expect(addDays("2025-01-31", 1)).toBe("2025-02-01");
  });

  it("addDays 支持闰年 2 月边界", () => {
    expect(addDays("2024-02-28", 1)).toBe("2024-02-29");
    expect(addDays("2024-02-29", 1)).toBe("2024-03-01");
  });

  it("startOfWeekYmd：周一为起点（覆盖周一到周日）", () => {
    // 2025-01-06 是周一，2025-01-12 是周日
    expect(startOfWeekYmd("2025-01-06")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-07")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-08")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-09")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-10")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-11")).toBe("2025-01-06");
    expect(startOfWeekYmd("2025-01-12")).toBe("2025-01-06");
  });

  it("startOfMonthYmd：返回每月首日", () => {
    expect(startOfMonthYmd("2025-01")).toBe("2025-01-01");
    expect(startOfMonthYmd("2025-12")).toBe("2025-12-01");
  });

  it("endOfMonthYmd：返回每月末日（含闰年 2 月）", () => {
    expect(endOfMonthYmd("2025-01")).toBe("2025-01-31");
    expect(endOfMonthYmd("2025-02")).toBe("2025-02-28");
    expect(endOfMonthYmd("2024-02")).toBe("2024-02-29");
  });
});
