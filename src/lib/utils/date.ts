/** 日期范围与 `YYYY-MM-DD` 处理的通用工具（仅依赖浏览器 Date）。 */

/**
 * 将 `YYYY-MM-DD` 解析为 Date（本地时区，00:00）。
 */
export function parseYmd(ymd: string): Date {
  const [y, m, d] = ymd.split("-").map((x) => Number(x));
  return new Date(y, (m ?? 1) - 1, d ?? 1, 0, 0, 0, 0);
}

/**
 * 将 Date 格式化为 `YYYY-MM-DD`。
 */
export function formatYmd(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * 获取今天日期（YYYY-MM-DD）。
 */
export function todayYmd(): string {
  return formatYmd(new Date());
}

/**
 * 日期加减天数。
 */
export function addDays(ymd: string, delta: number): string {
  const dt = parseYmd(ymd);
  dt.setDate(dt.getDate() + delta);
  return formatYmd(dt);
}

/**
 * 获取周一作为起始的周范围起点（YYYY-MM-DD）。
 */
export function startOfWeekYmd(ymd: string): string {
  const dt = parseYmd(ymd);
  const day = dt.getDay(); // Sun=0, Mon=1...
  const offset = day === 0 ? 6 : day - 1;
  dt.setDate(dt.getDate() - offset);
  return formatYmd(dt);
}

/**
 * 获取指定月份的第一天（YYYY-MM-DD）。
 */
export function startOfMonthYmd(ym: string): string {
  const [y, m] = ym.split("-").map((x) => Number(x));
  return formatYmd(new Date(y, (m ?? 1) - 1, 1));
}

/**
 * 获取指定月份的最后一天（YYYY-MM-DD）。
 */
export function endOfMonthYmd(ym: string): string {
  const [y, m] = ym.split("-").map((x) => Number(x));
  return formatYmd(new Date(y, (m ?? 1), 0));
}

