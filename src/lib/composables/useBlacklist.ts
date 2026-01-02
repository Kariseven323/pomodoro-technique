/** 黑名单相关组合式工具：封装保存黑名单与模板变更同步。 */

import { get } from "svelte/store";
import { setBlacklist } from "$lib/api/tauri";
import { appData } from "$lib/stores/appClient";
import type { AppData, BlacklistItem, BlacklistTemplate } from "$lib/shared/types";

/** `useBlacklist` 依赖注入参数。 */
export interface UseBlacklistDeps {
  /** 展示提示信息的方法（由调用方决定 UI 形态）。 */
  showToast: (message: string) => void;
}

/** 黑名单模板变更事件负载（与 `BlacklistModal` 对齐）。 */
export interface TemplatesChangeDetail {
  /** 最新模板列表。 */
  templates: BlacklistTemplate[];
  /** 最新启用模板 id 列表。 */
  activeTemplateIds: string[];
  /** 应用模板后的黑名单。 */
  blacklist: BlacklistItem[];
}

/** `useBlacklist` 返回的操作集合。 */
export interface UseBlacklistApi {
  /** 保存黑名单到后端并同步到全局 store。 */
  saveBlacklist: (next: BlacklistItem[]) => Promise<boolean>;
  /** 将模板变更写回全局 store（不触发后端调用）。 */
  applyTemplatesChange: (detail: TemplatesChangeDetail) => void;
}

/**
 * 提供黑名单相关的业务操作，并统一处理错误提示与全局状态同步。
 */
export function useBlacklist(deps: UseBlacklistDeps): UseBlacklistApi {
  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 保存黑名单并在成功后写回 `appData.blacklist`。 */
  async function saveBlacklist(next: BlacklistItem[]): Promise<boolean> {
    const current = get(appData);
    if (!current) return false;
    try {
      const saved = await setBlacklist(next);
      appData.set({ ...current, blacklist: saved });
      deps.showToast("黑名单已更新");
      return true;
    } catch (e) {
      deps.showToast(formatError(e));
      return false;
    }
  }

  /** 将模板变更写回全局 `AppData`（用于 UI 即时反映模板状态）。 */
  function applyTemplatesChange(detail: TemplatesChangeDetail): void {
    const current = get(appData);
    if (!current) return;
    const activeTemplateId = detail.activeTemplateIds[0] ?? null;
    const next: AppData = {
      ...current,
      blacklistTemplates: detail.templates,
      activeTemplateIds: detail.activeTemplateIds,
      activeTemplateId,
      blacklist: detail.blacklist,
    };
    appData.set(next);
  }

  return { saveBlacklist, applyTemplatesChange };
}
