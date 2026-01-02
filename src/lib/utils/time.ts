/** 时间与倒计时相关的通用格式化工具。 */

/**
 * 将秒数格式化为 `mm:ss`（分钟上限显示到 99）。
 */
export function formatMmSs(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${String(Math.min(minutes, 99)).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

