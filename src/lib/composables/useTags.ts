/** 标签相关组合式工具：封装标签选择与新增逻辑。 */

import { setCurrentTag } from "$lib/api/tauri";
import { applyAppSnapshot } from "$lib/stores/appClient";

/** `useTags` 依赖注入参数。 */
export interface UseTagsDeps {
  /** 展示提示信息的方法（由调用方决定 UI 形态）。 */
  showToast: (message: string) => void;
}

/** `useTags` 返回的操作集合。 */
export interface UseTagsApi {
  /** 选择现有标签。 */
  onTagSelectChange: (tag: string) => Promise<void>;
  /** 新建并选择标签。 */
  addAndSelectTag: (tag: string) => Promise<void>;
}

/**
 * 提供标签相关的业务操作，并统一处理错误提示与快照同步。
 */
export function useTags(deps: UseTagsDeps): UseTagsApi {
  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 选择一个已有标签并同步到后端。 */
  async function onTagSelectChange(tag: string): Promise<void> {
    const nextTag = tag.trim();
    if (!nextTag) return;
    try {
      const snapshot = await setCurrentTag(nextTag);
      applyAppSnapshot(snapshot);
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  /** 新建标签并立即选中（由后端负责自动加入标签历史）。 */
  async function addAndSelectTag(tag: string): Promise<void> {
    const nextTag = tag.trim();
    if (!nextTag) return;
    try {
      const snapshot = await setCurrentTag(nextTag);
      applyAppSnapshot(snapshot);
      deps.showToast("已添加标签");
    } catch (e) {
      deps.showToast(formatError(e));
    }
  }

  return { onTagSelectChange, addAndSelectTag };
}
