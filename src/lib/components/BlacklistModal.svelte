<script lang="ts">
  import type { BlacklistItem, ProcessInfo } from "$lib/types";
  import { listProcesses } from "$lib/tauriApi";
  import { createEventDispatcher } from "svelte";

  const props = $props<{ open: boolean; blacklist: BlacklistItem[]; locked: boolean }>();

  const dispatch = createEventDispatcher<{
    close: void;
    save: BlacklistItem[];
  }>();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state("");
  let processes = $state<ProcessInfo[]>([]);
  let lastUpdatedAt = $state<number | null>(null);
  let draft = $state<BlacklistItem[]>([]);
  let originalNames = $state<string[]>([]);
  let autoRefreshTimerId: number | null = null;
  let autoRefreshCleanup: (() => void) | null = null;

  /** 正在运行进程列表自动刷新间隔（毫秒）。 */
  const AUTO_REFRESH_INTERVAL_MS = 2000;

  /** 规范化进程名用于比较（忽略大小写与首尾空白）。 */
  function normalizeName(name: string): string {
    return name.trim().toLowerCase();
  }

  /** 将 `blacklist` 同步到本地草稿。 */
  function syncDraftFromProps(): void {
    const nextDraft: BlacklistItem[] = [];
    const nextOriginal: string[] = [];
    for (const b of props.blacklist) {
      nextDraft.push({ ...b });
      nextOriginal.push(normalizeName(b.name));
    }
    draft = nextDraft;
    originalNames = nextOriginal;
  }

  /** 关闭弹窗（不保存）。 */
  function closeModal(): void {
    dispatch("close");
  }

  /** 从后端加载进程列表（避免并发请求导致 UI 抖动）。 */
  async function loadProcesses(): Promise<void> {
    if (loading) return;
    loading = true;
    error = null;
    try {
      processes = await listProcesses();
      lastUpdatedAt = Date.now();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  /** 立即刷新进程列表（用于按钮点击/聚焦回到窗口时）。 */
  function refreshProcessesNow(): void {
    void loadProcesses();
  }

  /** 停止自动刷新并清理事件监听。 */
  function stopAutoRefresh(): void {
    if (autoRefreshTimerId !== null) {
      window.clearInterval(autoRefreshTimerId);
      autoRefreshTimerId = null;
    }
    if (autoRefreshCleanup) {
      autoRefreshCleanup();
      autoRefreshCleanup = null;
    }
  }

  /** 启动自动刷新：打开弹窗时持续同步“正在运行的进程”，避免页面停留导致列表过期。 */
  function startAutoRefresh(): void {
    stopAutoRefresh();
    autoRefreshTimerId = window.setInterval(refreshProcessesNow, AUTO_REFRESH_INTERVAL_MS);

    /** 窗口重新获得焦点时立即刷新，减少用户“切回应用后仍是旧列表”的感知延迟。 */
    const onFocus = (): void => {
      refreshProcessesNow();
    };
    /** 页面从后台切回前台时刷新（例如最小化/切换窗口回来）。 */
    const onVisibilityChange = (): void => {
      if (!document.hidden) refreshProcessesNow();
    };

    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibilityChange);

    /** 清理本次启动的事件监听（由 `stopAutoRefresh` 调用）。 */
    autoRefreshCleanup = (): void => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }

  /** 判断草稿黑名单是否包含指定进程名。 */
  function hasInDraft(name: string): boolean {
    const key = normalizeName(name);
    for (const b of draft) {
      if (normalizeName(b.name) === key) return true;
    }
    return false;
  }

  /** 获取草稿中的显示名（不存在时返回推荐值）。 */
  function getDisplayName(name: string): string {
    const key = normalizeName(name);
    for (const b of draft) {
      if (normalizeName(b.name) === key) return b.displayName;
    }
    return name.replace(/\\.exe$/i, "");
  }

  /** 更新草稿中的显示名。 */
  function setDisplayName(name: string, displayName: string): void {
    const key = normalizeName(name);
    let idx = -1;
    for (let i = 0; i < draft.length; i += 1) {
      if (normalizeName(draft[i].name) === key) {
        idx = i;
        break;
      }
    }
    if (idx >= 0) {
      draft[idx] = { ...draft[idx], displayName };
    }
  }

  /** 向草稿黑名单中添加进程。 */
  function addToDraft(name: string): void {
    if (hasInDraft(name)) return;
    draft = [...draft, { name, displayName: getDisplayName(name) }];
  }

  /** 从草稿黑名单中移除进程（专注期锁定时禁止移除原有条目）。 */
  function removeFromDraft(name: string): void {
    const key = normalizeName(name);
    if (props.locked && originalNames.includes(key)) {
      return;
    }
    const next: BlacklistItem[] = [];
    for (const b of draft) {
      if (normalizeName(b.name) !== key) next.push(b);
    }
    draft = next;
  }

  /** 切换运行进程勾选状态（添加/移除）。 */
  function toggleProcess(name: string, checked: boolean): void {
    if (checked) {
      addToDraft(name);
    } else {
      removeFromDraft(name);
    }
  }

  /** 保存草稿并关闭弹窗。 */
  function saveAndClose(): void {
    const out: BlacklistItem[] = [];
    for (const b of draft) {
      out.push({ name: b.name.trim(), displayName: b.displayName.trim() });
    }
    dispatch("save", out);
  }

  /** 响应 `open` 变化：打开时加载并同步，关闭时清理状态。 */
  function onOpenEffect(): void {
    if (props.open) {
      syncDraftFromProps();
      refreshProcessesNow();
      startAutoRefresh();
    } else {
      stopAutoRefresh();
      query = "";
      error = null;
    }
  }

  $effect(onOpenEffect);

  /** 按查询过滤进程列表。 */
  function filteredProcesses(): ProcessInfo[] {
    const q = query.trim().toLowerCase();
    if (!q) return processes;
    const out: ProcessInfo[] = [];
    for (const p of processes) {
      if (p.name.toLowerCase().includes(q)) out.push(p);
    }
    return out;
  }

  /** 处理“显示名”输入框变化（通过 `data-name` 定位条目）。 */
  function onDisplayNameInput(e: Event): void {
    const el = e.currentTarget as HTMLInputElement;
    const name = el.dataset["name"] ?? "";
    setDisplayName(name, el.value);
  }

  /** 处理“移除”按钮点击（通过 `data-name` 定位条目）。 */
  function onRemoveClick(e: Event): void {
    const el = e.currentTarget as HTMLButtonElement;
    const name = el.dataset["name"] ?? "";
    removeFromDraft(name);
  }

  /** 处理进程勾选变化（通过 `data-name` 定位进程）。 */
  function onProcessToggle(e: Event): void {
    const el = e.currentTarget as HTMLInputElement;
    const name = el.dataset["name"] ?? "";
    toggleProcess(name, el.checked);
  }
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button
      type="button"
      class="absolute inset-0 bg-black/30 backdrop-blur-sm"
      aria-label="关闭弹窗"
      onclick={closeModal}
    ></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div
        class="w-full max-w-3xl rounded-3xl border border-white/20 bg-white/80 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/70"
      >
        <div class="mb-4 flex items-center justify-between gap-3">
          <div>
            <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-50">管理黑名单</h2>
            <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">
              {#if props.locked}
                专注期内仅允许新增，禁止移除已存在条目。
              {:else}
                可勾选/取消勾选后保存生效。
              {/if}
            </p>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="rounded-xl px-3 py-1 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
              onclick={closeModal}
            >
              关闭
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
            <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">已加入黑名单</div>
            {#if draft.length === 0}
              <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无黑名单</div>
            {:else}
              <div class="max-h-80 space-y-2 overflow-auto pr-1">
                {#each draft as item (item.name)}
                  <div class="flex items-center gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
                    <div class="min-w-0 flex-1">
                      <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{item.name}</div>
                      <input
                        class="mt-1 w-full rounded-xl border border-black/10 bg-white/70 px-2 py-1 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                        value={item.displayName}
                        data-name={item.name}
                        oninput={onDisplayNameInput}
                        placeholder="显示名"
                      />
                    </div>
                    <button
                      class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/10 disabled:opacity-40 dark:text-zinc-200 dark:hover:bg-white/10"
                      disabled={props.locked && originalNames.includes(normalizeName(item.name))}
                      data-name={item.name}
                      onclick={onRemoveClick}
                    >
                      移除
                    </button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
            <div class="mb-2 flex items-center justify-between gap-2">
              <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">正在运行的进程</div>
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/5 disabled:opacity-40 dark:text-zinc-300 dark:hover:bg-white/10"
                  disabled={loading}
                  onclick={refreshProcessesNow}
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
            {#if lastUpdatedAt}
              <div class="mb-2 text-[11px] text-zinc-500 dark:text-zinc-400">
                最近刷新：{new Date(lastUpdatedAt).toLocaleTimeString()}
              </div>
            {/if}

            {#if error}
              <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">
                加载失败：{error}
              </div>
            {:else if loading}
              <div class="text-sm text-zinc-500 dark:text-zinc-400">正在加载进程...</div>
            {:else}
              <div class="max-h-80 space-y-2 overflow-auto pr-1">
                {#each filteredProcesses() as p (p.name)}
                  <label class="flex items-center gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
                    <input
                      class="h-4 w-4"
                      type="checkbox"
                      checked={hasInDraft(p.name)}
                      data-name={p.name}
                      onchange={onProcessToggle}
                    />
                    {#if p.iconDataUrl}
                      <img class="h-5 w-5 rounded" src={p.iconDataUrl} alt="" />
                    {:else}
                      <div class="h-5 w-5 rounded bg-white/60 dark:bg-white/20"></div>
                    {/if}
                    <div class="min-w-0 flex-1">
                      <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{p.name}</div>
                      <div class="truncate text-[11px] text-zinc-500 dark:text-zinc-400">
                        PID: {p.pid}{#if p.exePath} · {p.exePath}{/if}
                      </div>
                    </div>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class="mt-5 flex items-center justify-end gap-2">
          <button
            class="rounded-2xl px-4 py-2 text-sm text-zinc-700 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            取消
          </button>
          <button
            class="rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
            onclick={saveAndClose}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
