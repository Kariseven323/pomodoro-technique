/** 设置相关组合式工具：封装保存设置与窗口置顶切换。 */

import { get } from "svelte/store";
import { setAlwaysOnTop, updateSettings } from "$lib/api/tauri";
import { appData, applyAppSnapshot } from "$lib/stores/appClient";
import type { Settings } from "$lib/shared/types";

/** `useSettings` 依赖注入参数。 */
export interface UseSettingsDeps {
  /** 展示提示信息的方法（由调用方决定 UI 形态）。 */
  showToast: (message: string) => void;
}

/** `useSettings` 返回的操作集合。 */
export interface UseSettingsApi {
  /** 保存设置（成功返回 `true`，失败返回 `false`）。 */
  saveSettings: (next: Settings) => Promise<boolean>;
  /** 切换主窗口置顶。 */
  toggleAlwaysOnTop: () => Promise<void>;
}

/**
 * 提供设置相关的业务操作，并统一处理错误提示与全局状态同步。
 */
export function useSettings(deps: UseSettingsDeps): UseSettingsApi {
  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 保存设置并同步到全局 store；需要时额外调用 `setAlwaysOnTop`。 */
  async function saveSettings(next: Settings): Promise<boolean> {
    const current = get(appData);
    if (!current) return false;
    const prevAlwaysOnTop = current.settings.alwaysOnTop;
    try {
      const snapshot = await updateSettings(next);
      applyAppSnapshot(snapshot);
      if (next.alwaysOnTop !== prevAlwaysOnTop) {
        await setAlwaysOnTop(next.alwaysOnTop);
      }
      deps.showToast("已保存设置");
      return true;
    } catch (e) {
      deps.showToast(formatError(e));
      return false;
    }
  }

  /** 切换置顶并写回全局状态。 */
  async function toggleAlwaysOnTop(): Promise<void> {
    const current = get(appData);
    if (!current) return;
    const next = !current.settings.alwaysOnTop;
    try {
      await setAlwaysOnTop(next);
      appData.set({ ...current, settings: { ...current.settings, alwaysOnTop: next } });
      deps.showToast(next ? "已置顶" : "已取消置顶");
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  return { saveSettings, toggleAlwaysOnTop };
}
