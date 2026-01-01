<script lang="ts">
  import { debugClearHistory, debugGenerateHistory } from "$lib/tauriApi";

  let days = $state<number>(30);
  let loading = $state(false);
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);

  /** 规范化输入天数到 1-365。 */
  function clampDays(value: number): number {
    if (!Number.isFinite(value)) return 30;
    return Math.max(1, Math.min(365, Math.floor(value)));
  }

  /** 生成测试数据：弹确认框后调用后端生成命令。 */
  async function onGenerateClick(): Promise<void> {
    error = null;
    notice = null;
    const nextDays = clampDays(days);
    days = nextDays;
    const ok = window.confirm(`确认生成最近 ${nextDays} 天的测试历史数据吗？（将写入 history_dev）`);
    if (!ok) return;
    loading = true;
    try {
      const count = await debugGenerateHistory(nextDays);
      notice = `已生成测试记录 ${count} 条（${nextDays} 天）`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  /** 清除测试数据：弹确认框后调用后端清除命令。 */
  async function onClearClick(): Promise<void> {
    error = null;
    notice = null;
    const ok = window.confirm("确认清除所有测试历史数据吗？（仅清空 history_dev）");
    if (!ok) return;
    loading = true;
    try {
      await debugClearHistory();
      notice = "已清除测试历史数据";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
  <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">开发者选项</div>
  <div class="text-xs text-zinc-600 dark:text-zinc-300">
    调试模式：一键生成/清除测试历史数据，用于验证历史页 UI（仅开发环境可见）。
  </div>

  {#if notice}
    <div class="mt-2 rounded-2xl bg-emerald-500/10 p-3 text-xs text-emerald-700 dark:text-emerald-300">{notice}</div>
  {/if}
  {#if error}
    <div class="mt-2 rounded-2xl bg-red-500/10 p-3 text-xs text-red-600 dark:text-red-300">失败：{error}</div>
  {/if}

  <div class="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-3">
    <label class="block sm:col-span-1">
      <div class="mb-1 text-xs text-zinc-600 dark:text-zinc-300">生成天数（1-365）</div>
      <input
        class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
        type="number"
        min="1"
        max="365"
        bind:value={days}
        disabled={loading}
      />
    </label>
    <div class="flex items-end gap-2 sm:col-span-2">
      <button
        type="button"
        class="flex-1 rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
        disabled={loading}
        onclick={() => void onGenerateClick()}
      >
        {loading ? "处理中..." : "生成测试数据"}
      </button>
      <button
        type="button"
        class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm text-zinc-800 shadow-sm hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
        disabled={loading}
        onclick={() => void onClearClick()}
      >
        清除测试数据
      </button>
    </div>
  </div>
</div>

