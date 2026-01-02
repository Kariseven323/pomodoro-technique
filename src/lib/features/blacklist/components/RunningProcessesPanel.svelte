<script lang="ts">
  import type { ProcessInfo } from "$lib/shared/types";
  import { frontendLog, processIcon } from "$lib/api/tauri";

  const props = $props<{
    /** 后端返回的进程列表。 */
    processes: ProcessInfo[];
    /** 是否处于“首次加载”（为空列表时显示加载态）。 */
    initialLoading: boolean;
    /** 是否正在后台刷新（不遮挡列表，避免无法操作）。 */
    refreshing: boolean;
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
  let iconByExePath = $state<Record<string, string | null>>({});
  let iconLoadingExePaths = $state<Record<string, boolean>>({});
  let iconErrorLoggedCount = $state(0);
  let listEl = $state<HTMLDivElement | null>(null);
  let listScrollTop = $state(0);

  /** 近似的单行条目“步进高度”（包含条目高度与纵向间距），用于按滚动位置推导可见范围。 */
  const ITEM_STEP_PX = 56;
  /** 在可见范围之外额外预加载的条目数量（提升滚动时图标命中率）。 */
  const ICON_PREFETCH_BUFFER = 12;

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

  /** 获取某个进程的“可用图标 data URL”（优先使用后端列表自带，其次使用按需加载缓存）。 */
  function iconDataUrlOf(p: ProcessInfo): string | null {
    if (p.iconDataUrl) return p.iconDataUrl;
    const exe = p.exePath?.trim();
    if (!exe) return null;
    return iconByExePath[exe] ?? null;
  }

  /** 按需加载单个 exe 图标，并写入本地缓存（失败时缓存为 null，避免重复请求）。 */
  async function loadIconForExePath(exePath: string): Promise<void> {
    const key = exePath.trim();
    if (!key) return;
    if (iconByExePath[key] !== undefined) return;
    if (iconLoadingExePaths[key]) return;
    iconLoadingExePaths = { ...iconLoadingExePaths, [key]: true };
    try {
      const out = await processIcon(key);
      iconByExePath = { ...iconByExePath, [key]: out };
    } catch (e) {
      iconByExePath = { ...iconByExePath, [key]: null };
      // 记录少量诊断日志，帮助定位“图标一直加载不到”的原因（避免大量进程导致日志刷屏）。
      if (iconErrorLoggedCount < 3) {
        iconErrorLoggedCount += 1;
        void frontendLog("warn", `processIcon 失败：exePath=${key} err=${e instanceof Error ? e.message : String(e)}`);
      }
    } finally {
      const next = { ...iconLoadingExePaths };
      delete next[key];
      iconLoadingExePaths = next;
    }
  }

  /** 计算当前滚动位置下“应尝试加载图标”的进程切片（仅针对可见区域，避免一次性触发过多请求）。 */
  function processesInViewportSlice(): ProcessInfo[] {
    const list = filteredProcesses();
    if (list.length === 0) return [];
    const height = listEl?.clientHeight ?? 0;
    const perView = height > 0 ? Math.max(1, Math.ceil(height / ITEM_STEP_PX)) : 12;
    const start = Math.max(0, Math.floor(listScrollTop / ITEM_STEP_PX));
    const end = Math.min(list.length, start + perView + ICON_PREFETCH_BUFFER);
    return list.slice(start, end);
  }

  /** 自动为“当前可见区域”按需加载图标，避免首屏传输大 payload 卡死。 */
  function onAutoLoadIconsEffect(): void {
    if (props.initialLoading || props.error) return;
    const list = processesInViewportSlice();
    for (const p of list) {
      if (iconDataUrlOf(p)) continue;
      const exe = p.exePath?.trim();
      if (!exe) continue;
      void loadIconForExePath(exe);
    }
  }

  $effect(onAutoLoadIconsEffect);

  /** 记录列表滚动位置（用于按需加载当前可见区域的图标）。 */
  function onListScroll(e: Event): void {
    const el = e.currentTarget as HTMLDivElement;
    listScrollTop = el.scrollTop;
  }

  /** 手动刷新按钮点击（集中处理，便于后续补充交互逻辑）。 */
  function onRefreshClick(): void {
    props.onRefresh();
  }
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
  <div class="mb-2 flex items-center justify-between gap-2">
    <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">正在运行的进程</div>
  </div>

  {#if props.lastUpdatedAt}
    <div class="mb-2 text-[11px] text-zinc-500 dark:text-zinc-400">
      最近刷新：{new Date(props.lastUpdatedAt).toLocaleTimeString()}{#if props.refreshing}
        · 刷新中...{/if}
    </div>
  {/if}

  <div class="mb-2 flex items-center gap-2">
    <button
      type="button"
      class="rounded-xl px-3 py-2 text-xs text-zinc-700 hover:bg-black/5 disabled:opacity-40 dark:text-zinc-200 dark:hover:bg-white/10"
      disabled={props.initialLoading || props.refreshing}
      onclick={onRefreshClick}
    >
      刷新
    </button>
    <input
      class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      placeholder="搜索进程..."
      bind:value={query}
    />
  </div>

  {#if props.error}
    <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">加载失败：{props.error}</div>
  {:else}
    <div bind:this={listEl} class="max-h-80 space-y-2 overflow-auto pr-1" onscroll={onListScroll}>
      {#each filteredProcesses() as p (p.name)}
        <label class="flex h-12 items-center gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
          <input
            class="h-4 w-4"
            type="checkbox"
            checked={props.hasInDraft(p.name)}
            onchange={(e) => onProcessToggle(p.name, e)}
          />
          {#if iconDataUrlOf(p)}
            <img class="h-5 w-5 rounded" src={iconDataUrlOf(p) ?? ""} alt="" />
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

  {#if props.initialLoading && props.processes.length === 0 && !props.error}
    <div class="mt-2 text-sm text-zinc-500 dark:text-zinc-400">正在加载进程...</div>
  {/if}
</div>
