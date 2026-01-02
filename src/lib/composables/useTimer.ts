/** 计时器相关组合式工具：封装开始/暂停、重置与跳过等操作。 */

import { get } from "svelte/store";
import { timerPause, timerReset, timerSkip, timerStart } from "$lib/api/tauri";
import { timerSnapshot } from "$lib/stores/appClient";

/** `useTimer` 依赖注入参数。 */
export interface UseTimerDeps {
  /** 展示提示信息的方法（由调用方决定 UI 形态）。 */
  showToast: (message: string) => void;
}

/** `useTimer` 返回的操作集合。 */
export interface UseTimerApi {
  /** 切换开始/暂停。 */
  toggleStartPause: () => Promise<void>;
  /** 重置计时器。 */
  resetTimer: () => Promise<void>;
  /** 跳过当前阶段。 */
  skipTimer: () => Promise<void>;
}

/**
 * 提供计时器操作方法，并统一处理错误提示。
 */
export function useTimer(deps: UseTimerDeps): UseTimerApi {
  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 切换开始/暂停（根据当前快照判断）。 */
  async function toggleStartPause(): Promise<void> {
    const snapshot = get(timerSnapshot);
    if (!snapshot) return;
    try {
      await (snapshot.isRunning ? timerPause() : timerStart());
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  /** 重置计时器。 */
  async function resetTimer(): Promise<void> {
    try {
      await timerReset();
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  /** 跳过当前阶段。 */
  async function skipTimer(): Promise<void> {
    try {
      await timerSkip();
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  return { toggleStartPause, resetTimer, skipTimer };
}
