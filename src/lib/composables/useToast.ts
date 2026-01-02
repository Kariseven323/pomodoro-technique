/** Toast 组合式工具：提供可复用的提示状态与显示/清理方法。 */

import { writable, type Writable } from "svelte/store";

/** Toast 组合式工具的返回值。 */
export interface ToastApi {
  /** 当前 toast 文本（`null` 表示不显示）。 */
  toast: Writable<string | null>;
  /** 展示一条 toast，并在超时后自动隐藏。 */
  showToast: (message: string) => void;
  /** 立即清空 toast。 */
  clearToast: () => void;
}

/** Toast 组合式工具的可选配置。 */
export interface ToastOptions {
  /** 自动隐藏时长（毫秒）。 */
  autoHideMs?: number;
}

/**
 * 创建一组独立的 toast 状态与控制方法（适合页面/弹窗局部使用）。
 */
export function useToast(options: ToastOptions = {}): ToastApi {
  const toast = writable<string | null>(null);
  const autoHideMs = options.autoHideMs ?? 2200;
  let timerId: number | null = null;

  /** 清理自动隐藏的定时器，避免多次 `showToast` 互相覆盖。 */
  function clearTimer(): void {
    if (timerId === null) return;
    window.clearTimeout(timerId);
    timerId = null;
  }

  /** 立即清空 toast，并取消自动隐藏定时器。 */
  function clearToast(): void {
    clearTimer();
    toast.set(null);
  }

  /** 展示一条 toast，并在 `autoHideMs` 后自动清空。 */
  function showToast(message: string): void {
    clearTimer();
    toast.set(message);
    timerId = window.setTimeout(clearToast, autoHideMs);
  }

  return { toast, showToast, clearToast };
}
