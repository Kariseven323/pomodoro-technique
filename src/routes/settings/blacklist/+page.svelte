<script lang="ts">
  import { onMount } from "svelte";
  import { appData, killSummary, timerSnapshot } from "$lib/stores/appClient";
  import { listProcesses, restartAsAdmin } from "$lib/api/tauri";
  import type { BlacklistItem, BlacklistTemplate, ProcessInfo } from "$lib/shared/types";
  import BlacklistDraftList from "$lib/features/blacklist/components/BlacklistDraftList.svelte";
  import RunningProcessesPanel from "$lib/features/blacklist/components/RunningProcessesPanel.svelte";
  import TemplateManager from "$lib/features/blacklist/components/TemplateManager.svelte";
  import { useBlacklist } from "$lib/composables/useBlacklist";
  import { useToast } from "$lib/composables/useToast";

  const { toast: toastMessage, showToast } = useToast();
  const blacklist = useBlacklist({ showToast });

  let initialLoading = $state(false);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let processes = $state<ProcessInfo[]>([]);
  let lastUpdatedAt = $state<number | null>(null);

  let draft = $state<BlacklistItem[]>([]);
  let originalNames = $state<string[]>([]);

  /** 当前是否处于“专注期锁定”（只能新增不能移除原有条目）。 */
  const locked = $derived($timerSnapshot?.blacklistLocked ?? false);

  /** 规范化进程名用于比较（忽略大小写与首尾空白）。 */
  function normalizeName(name: string): string {
    return name.trim().toLowerCase();
  }

  /** 将全局 `blacklist` 同步到本地草稿。 */
  function syncDraftFromStore(): void {
    const current = $appData;
    if (!current) return;
    const nextDraft: BlacklistItem[] = [];
    const nextOriginal: string[] = [];
    for (const b of current.blacklist) {
      nextDraft.push({ ...b });
      nextOriginal.push(normalizeName(b.name));
    }
    draft = nextDraft;
    originalNames = nextOriginal;
  }

  /** 从后端加载进程列表（避免并发请求导致 UI 抖动）。 */
  async function loadProcesses(): Promise<void> {
    if (initialLoading || refreshing) return;
    const isInitial = processes.length === 0;
    if (isInitial) {
      initialLoading = true;
    } else {
      refreshing = true;
    }
    error = null;
    try {
      processes = await listProcesses();
      lastUpdatedAt = Date.now();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  /** 立即刷新进程列表（用于按钮点击）。 */
  function refreshProcessesNow(): void {
    void loadProcesses();
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
    if (locked && originalNames.includes(key)) {
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

  /** 判断某条目是否为进入页面时“已有条目”（用于锁定时禁用移除）。 */
  function isOriginalName(name: string): boolean {
    return originalNames.includes(normalizeName(name));
  }

  /** 保存草稿到后端并刷新全局 store。 */
  async function save(): Promise<void> {
    const out: BlacklistItem[] = [];
    for (const b of draft) {
      out.push({ name: b.name.trim(), displayName: b.displayName.trim() });
    }
    await blacklist.saveBlacklist(out);
    syncDraftFromStore();
  }

  /** 处理模板管理器推送的模板变更：同步草稿并写回全局 store。 */
  function onTemplatesChange(
    e: CustomEvent<{ templates: BlacklistTemplate[]; activeTemplateIds: string[]; blacklist: BlacklistItem[] }>,
  ): void {
    draft = e.detail.blacklist.map((b) => ({ ...b }));
    originalNames = draft.map((b) => normalizeName(b.name));
    blacklist.applyTemplatesChange(e.detail);
  }

  /** 请求以管理员身份重启（用于终止需要提权的进程）。 */
  async function onRestartAsAdmin(): Promise<void> {
    try {
      await restartAsAdmin();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** Svelte 生命周期：进入页面时初始化草稿并刷新进程列表。 */
  function onMounted(): void {
    syncDraftFromStore();
    void loadProcesses();
  }

  onMount(onMounted);
</script>

<main class="min-h-screen bg-zinc-50 p-4 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50">
  <div class="mx-auto w-full max-w-5xl">
    <header class="mb-4 flex items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <a
          href="/settings"
          class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
        >
          ← 返回
        </a>
        <div>
          <h1 class="text-lg font-semibold tracking-tight">黑名单管理</h1>
          <p class="mt-1 text-xs text-zinc-500 dark:text-zinc-400">
            {#if locked}
              专注期内仅允许新增，禁止移除已存在条目。
            {:else}
              可勾选/取消勾选后保存生效。
            {/if}
          </p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="rounded-2xl border border-black/10 bg-white px-4 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
          onclick={syncDraftFromStore}
        >
          重置草稿
        </button>
        <button
          type="button"
          class="rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
          onclick={() => void save()}
        >
          保存
        </button>
      </div>
    </header>

    {#if $toastMessage}
      <div class="mb-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
        {$toastMessage}
      </div>
    {/if}

    {#if $killSummary?.requiresAdmin}
      <div class="mb-4 rounded-2xl bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300">
        检测到部分进程需要管理员权限才能终止。
        <button class="ml-2 underline" onclick={() => void onRestartAsAdmin()}>以管理员身份重启</button>
      </div>
    {/if}

    <div class="min-h-0 space-y-4">
      <TemplateManager
        open={true}
        {locked}
        templates={$appData?.blacklistTemplates ?? []}
        activeTemplateIds={$appData?.activeTemplateIds ?? []}
        {draft}
        on:templatesChange={onTemplatesChange}
      />

      <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div class="rounded-2xl bg-white p-4 shadow-sm dark:bg-zinc-900">
          <BlacklistDraftList
            {draft}
            {locked}
            {isOriginalName}
            onDisplayNameChange={setDisplayName}
            onRemove={removeFromDraft}
          />
        </div>
        <div class="rounded-2xl bg-white p-4 shadow-sm dark:bg-zinc-900">
          <RunningProcessesPanel
            {processes}
            {initialLoading}
            {refreshing}
            {error}
            {lastUpdatedAt}
            {hasInDraft}
            onToggle={toggleProcess}
            onRefresh={refreshProcessesNow}
          />
        </div>
      </div>
    </div>
  </div>
</main>
