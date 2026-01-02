<script lang="ts">
  import type { StorePaths } from "$lib/shared/types";
  import { frontendLog, getStorePaths, openStoreDir as openStoreDirApi } from "$lib/api/tauri";

  const props = $props<{ open: boolean }>();

  let storePaths = $state<StorePaths | null>(null);
  let storePathsLoading = $state(false);
  let storePathsOpening = $state(false);
  let storePathsError = $state<string | null>(null);
  let storePathsNotice = $state<string | null>(null);

  /** 加载应用数据根目录路径（统一入口，用于设置页展示）。 */
  async function loadStorePaths(): Promise<void> {
    if (storePathsLoading) return;
    storePathsLoading = true;
    storePathsError = null;
    try {
      storePaths = await getStorePaths();
    } catch (e) {
      storePathsError = e instanceof Error ? e.message : String(e);
      storePaths = null;
    } finally {
      storePathsLoading = false;
    }
  }

  /** 点击“打开文件夹”时打开应用数据根目录（统一入口）。 */
  async function onOpenStoreDirClick(): Promise<void> {
    try {
      if (storePathsOpening) return;
      storePathsOpening = true;
      storePathsError = null;
      storePathsNotice = `正在请求打开文件夹...（${new Date().toLocaleTimeString()}）`;

      void frontendLog("info", `[open_store_dir] click storeDir=${storePaths?.storeDirPath ?? ""}`).catch(() => {});
      await openStoreDirApi();
      void frontendLog("info", "[open_store_dir] requested").catch(() => {});

      storePathsNotice = "已请求打开文件夹";
      window.setTimeout(clearStorePathsNotice, 2000);
    } catch (e) {
      storePathsError = e instanceof Error ? e.message : String(e);
      storePathsNotice = null;
      void frontendLog("error", `[open_store_dir] failed: ${storePathsError}`).catch(() => {});
    } finally {
      storePathsOpening = false;
    }
  }

  /** 清理“打开文件夹”提示文案（用于定时隐藏）。 */
  function clearStorePathsNotice(): void {
    storePathsNotice = null;
  }

  /** 响应弹窗打开：触发预加载；关闭时清理提示状态。 */
  function onOpenEffect(): void {
    if (props.open) {
      void loadStorePaths();
    } else {
      storePathsError = null;
      storePathsNotice = null;
    }
  }

  $effect(onOpenEffect);
</script>

<div class="mt-4 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
  <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">数据根目录（统一入口）</div>
  {#if storePathsNotice}
    <div class="mb-2 text-xs text-emerald-700 dark:text-emerald-300">{storePathsNotice}</div>
  {/if}
  {#if storePathsError}
    <div class="mb-2 text-xs text-red-600 dark:text-red-300">失败：{storePathsError}</div>
  {/if}
  <div class="flex items-center gap-2">
    <input
      class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      readonly
      value={storePaths?.storeDirPath ?? ""}
      placeholder={storePathsLoading ? "正在获取..." : "未获取到路径"}
    />
    <button
      type="button"
      class="shrink-0 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
      disabled={storePathsOpening}
      onclick={onOpenStoreDirClick}
    >
      打开文件夹
    </button>
  </div>
</div>
