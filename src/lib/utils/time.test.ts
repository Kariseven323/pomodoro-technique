/** `utils/time.ts` 的单元测试：覆盖倒计时格式化边界。 */

import { describe, expect, it } from "vitest";
import { formatMmSs } from "./time";

describe("formatMmSs", () => {
  it("格式化 0 秒", () => {
    expect(formatMmSs(0)).toBe("00:00");
  });

  it("格式化 59 秒", () => {
    expect(formatMmSs(59)).toBe("00:59");
  });

  it("格式化 60 秒", () => {
    expect(formatMmSs(60)).toBe("01:00");
  });

  it("格式化 99 分 59 秒", () => {
    expect(formatMmSs(99 * 60 + 59)).toBe("99:59");
  });

  it("超过 99 分时应钳制分钟到 99", () => {
    expect(formatMmSs(100 * 60)).toBe("99:00");
  });
});
