<script lang="ts">
  import type { ProcessInfo } from "$lib/shared/types";

  const props = $props<{
    /** 后端返回的进程列表。 */
    processes: ProcessInfo[];
    /** 是否正在加载。 */
    loading: boolean;
    /** 加载错误信息。 */
    error: string | null;
    /** 最近刷新时间戳（毫秒）。 */
    lastUpdatedAt: number | null;
    /** 判断某进程名是否已在草稿中。 */
    hasInDraft: (name: string) => boolean;
    /** 切换勾选（添加/移除）。 */
    onToggle: (name: string, checked: boolean) => void;
    /** 手动刷新。 */
    onRefresh: () => void;
  }>();

  let query = $state("");

  /** 按查询过滤进程列表。 */
  function filteredProcesses(): ProcessInfo[] {
    const q = query.trim().toLowerCase();
    if (!q) return props.processes;
    const out: ProcessInfo[] = [];
    for (const p of props.processes) {
      if (p.name.toLowerCase().includes(q)) out.push(p);
    }
    return out;
  }

  /** 处理进程勾选变化。 */
  function onProcessToggle(name: string, e: Event): void {
    const el = e.currentTarget as HTMLInputElement;
    props.onToggle(name, el.checked);
  }
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
  <div class="mb-2 flex items-center justify-between gap-2">
    <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">正在运行的进程</div>
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/5 disabled:opacity-40 dark:text-zinc-300 dark:hover:bg-white/10"
        disabled={props.loading}
        onclick={props.onRefresh}
      >
        刷新
      </button>
      <input
        class="w-44 rounded-2xl border border-black/10 bg-white/70 px-3 py-1 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
        placeholder="搜索进程..."
        bind:value={query}
      />
    </div>
  </div>

  {#if props.lastUpdatedAt}
    <div class="mb-2 text-[11px] text-zinc-500 dark:text-zinc-400">
      最近刷新：{new Date(props.lastUpdatedAt).toLocaleTimeString()}
    </div>
  {/if}

  {#if props.error}
    <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">加载失败：{props.error}</div>
  {:else if props.loading}
    <div class="text-sm text-zinc-500 dark:text-zinc-400">正在加载进程...</div>
  {:else}
    <div class="max-h-80 space-y-2 overflow-auto pr-1">
      {#each filteredProcesses() as p (p.name)}
        <label class="flex items-center gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
          <input
            class="h-4 w-4"
            type="checkbox"
            checked={props.hasInDraft(p.name)}
            onchange={(e) => onProcessToggle(p.name, e)}
          />
          {#if p.iconDataUrl}
            <img class="h-5 w-5 rounded" src={p.iconDataUrl} alt="" />
          {:else}
            <div class="h-5 w-5 rounded bg-white/60 dark:bg-white/20"></div>
          {/if}
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{p.name}</div>
            <div class="truncate text-[11px] text-zinc-500 dark:text-zinc-400">
              PID: {p.pid}{#if p.exePath}
                · {p.exePath}{/if}
            </div>
          </div>
        </label>
      {/each}
    </div>
  {/if}
</div>
