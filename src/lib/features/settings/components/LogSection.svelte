<script lang="ts">
  import { openLogDir as openLogDirApi } from "$lib/api/tauri";

  const props = $props<{ open: boolean }>();

  let logNotice = $state<string | null>(null);
  let logError = $state<string | null>(null);

  /** 点击“查看日志”时打开日志目录（文件管理器）。 */
  async function onOpenLogDirClick(): Promise<void> {
    logError = null;
    logNotice = null;
    try {
      await openLogDirApi();
      logNotice = "已请求打开日志目录";
      window.setTimeout(clearLogNotice, 2000);
    } catch (e) {
      logError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 清理日志提示文案（用于定时隐藏）。 */
  function clearLogNotice(): void {
    logNotice = null;
  }

  /** 响应弹窗关闭：清理提示状态。 */
  function onOpenEffect(): void {
    if (!props.open) {
      logError = null;
      logNotice = null;
    }
  }

  $effect(onOpenEffect);
</script>

<div class="mt-4 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
  <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">日志</div>
  {#if logNotice}
    <div class="mb-2 text-xs text-emerald-700 dark:text-emerald-300">{logNotice}</div>
  {/if}
  {#if logError}
    <div class="mb-2 text-xs text-red-600 dark:text-red-300">失败：{logError}</div>
  {/if}
  <button
    type="button"
    class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
    onclick={onOpenLogDirClick}
  >
    查看日志
  </button>
</div>
