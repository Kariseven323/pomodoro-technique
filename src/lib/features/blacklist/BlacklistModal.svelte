<script lang="ts">
  import type { BlacklistItem, BlacklistTemplate, ProcessInfo } from "$lib/shared/types";
  import { applyTemplate, deleteTemplate, getTemplates, listProcesses, saveTemplate } from "$lib/api/tauri";
  import { createEventDispatcher } from "svelte";
  import BlacklistDraftList from "$lib/features/blacklist/components/BlacklistDraftList.svelte";
  import RunningProcessesPanel from "$lib/features/blacklist/components/RunningProcessesPanel.svelte";
  import TemplateSelector from "$lib/features/blacklist/components/TemplateSelector.svelte";

  const props = $props<{
    open: boolean;
    blacklist: BlacklistItem[];
    locked: boolean;
    templates: BlacklistTemplate[];
    activeTemplateIds: string[];
  }>();

  const dispatch = createEventDispatcher<{
    close: void;
    save: BlacklistItem[];
    templatesChange: { templates: BlacklistTemplate[]; activeTemplateIds: string[]; blacklist: BlacklistItem[] };
  }>();

	  let loading = $state(false);
	  let error = $state<string | null>(null);
	  let processes = $state<ProcessInfo[]>([]);
  let lastUpdatedAt = $state<number | null>(null);
  let draft = $state<BlacklistItem[]>([]);
  let originalNames = $state<string[]>([]);
  let templatesDraft = $state<BlacklistTemplate[]>([]);
  let activeTemplateIdsDraft = $state<string[]>([]);
  let templateNotice = $state<string | null>(null);
  let templateError = $state<string | null>(null);
  let saveTemplateName = $state("");
  let quickTemplateId = $state<string>("");
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

  /** 将 `templates/activeTemplateIds` 同步到本地草稿（用于模板切换与管理）。 */
  function syncTemplatesFromProps(): void {
    templatesDraft = props.templates.map((t: BlacklistTemplate) => ({
      ...t,
      processes: t.processes.map((p: BlacklistItem) => ({ ...p })),
    }));
    activeTemplateIdsDraft = [...props.activeTemplateIds];
    quickTemplateId = templatesDraft[0]?.id ?? "";
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

  /** 从后端刷新模板列表（打开弹窗时使用）。 */
  async function loadTemplates(): Promise<void> {
    templateError = null;
    try {
      const tpls = await getTemplates();
      templatesDraft = tpls.map((t: BlacklistTemplate) => ({
        ...t,
        processes: t.processes.map((p: BlacklistItem) => ({ ...p })),
      }));
      quickTemplateId = templatesDraft[0]?.id ?? "";
      dispatch("templatesChange", { templates: templatesDraft, activeTemplateIds: activeTemplateIdsDraft, blacklist: draft });
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
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

  /** 判断模板是否已启用。 */
  function isTemplateActive(id: string): boolean {
    return activeTemplateIdsDraft.includes(id);
  }

  /** 切换模板启用状态（专注期内禁止）。 */
  async function toggleTemplate(id: string): Promise<void> {
    if (props.locked) return;
    templateError = null;
    templateNotice = null;
    try {
      const nextBlacklist = await applyTemplate(id);
      const nextActive = isTemplateActive(id)
        ? activeTemplateIdsDraft.filter((x) => x !== id)
        : [...activeTemplateIdsDraft, id];
      nextActive.sort();
      activeTemplateIdsDraft = nextActive;

      draft = nextBlacklist.map((b) => ({ ...b }));
      originalNames = draft.map((b) => normalizeName(b.name));

      dispatch("templatesChange", { templates: templatesDraft, activeTemplateIds: nextActive, blacklist: nextBlacklist });
      templateNotice = "已应用模板";
      window.setTimeout((): void => {
        templateNotice = null;
      }, 1800);
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 快速应用下拉选择的模板（等价于切换启用状态）。 */
  function applyQuickTemplate(): void {
    if (!quickTemplateId) return;
    void toggleTemplate(quickTemplateId);
  }

  /** 另存为新模板：将当前草稿黑名单保存为自定义模板。 */
  async function saveCurrentAsTemplate(): Promise<void> {
    const name = saveTemplateName.trim();
    if (!name) return;
    templateError = null;
    templateNotice = null;
    try {
      const created = await saveTemplate({ id: "", name, builtin: false, processes: draft.map((b) => ({ ...b })) });
      const next = templatesDraft.filter((t) => t.id !== created.id);
      next.push(created);
      templatesDraft = next;
      saveTemplateName = "";
      dispatch("templatesChange", { templates: templatesDraft, activeTemplateIds: activeTemplateIdsDraft, blacklist: draft });
      templateNotice = "已保存为新模板";
      window.setTimeout((): void => {
        templateNotice = null;
      }, 1800);
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 删除自定义模板（内置模板不可删除）。 */
  async function deleteTemplateById(id: string): Promise<void> {
    templateError = null;
    templateNotice = null;
    try {
      const ok = await deleteTemplate(id);
      if (!ok) return;
      templatesDraft = templatesDraft.filter((t) => t.id !== id);
      activeTemplateIdsDraft = activeTemplateIdsDraft.filter((x) => x !== id);
      dispatch("templatesChange", { templates: templatesDraft, activeTemplateIds: activeTemplateIdsDraft, blacklist: draft });
      templateNotice = "已删除模板";
      window.setTimeout((): void => {
        templateNotice = null;
      }, 1800);
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 响应 `open` 变化：打开时加载并同步，关闭时清理状态。 */
  function onOpenEffect(): void {
	    if (props.open) {
	      syncDraftFromProps();
	      syncTemplatesFromProps();
	      refreshProcessesNow();
	      void loadTemplates();
	      startAutoRefresh();
	    } else {
	      stopAutoRefresh();
	      error = null;
	      templateError = null;
	      templateNotice = null;
	    }
	  }

  $effect(onOpenEffect);

  /** 判断某条目是否为打开弹窗时“已有条目”（用于锁定时禁用移除）。 */
  function isOriginalName(name: string): boolean {
    return originalNames.includes(normalizeName(name));
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

        <div class="mb-4 rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 flex items-start justify-between gap-3">
            <div>
              <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">黑名单模板</div>
              <div class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">
                {#if props.locked}
                  专注期内禁止切换模板。
                {:else}
                  可同时启用多套模板；切换会自动填充黑名单。
                {/if}
              </div>
            </div>
            <button
              type="button"
              class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
              onclick={loadTemplates}
            >
              刷新模板
            </button>
          </div>

          {#if templateNotice}
            <div class="mb-2 text-xs text-emerald-700 dark:text-emerald-300">{templateNotice}</div>
          {/if}
          {#if templateError}
            <div class="mb-2 text-xs text-red-600 dark:text-red-300">失败：{templateError}</div>
          {/if}

          <div class="max-h-40 overflow-auto pr-1">
            <TemplateSelector
              templates={templatesDraft}
              activeTemplateIds={activeTemplateIdsDraft}
              locked={props.locked}
              on:toggle={(e) => void toggleTemplate(e.detail)}
              on:delete={(e) => void deleteTemplateById(e.detail)}
            />
          </div>

          <div class="mt-3 flex items-center gap-2">
            <select
              class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              bind:value={quickTemplateId}
              disabled={templatesDraft.length === 0 || props.locked}
            >
              {#each templatesDraft as t (t.id)}
                <option class="bg-white text-zinc-900" value={t.id}>{t.name}</option>
              {/each}
            </select>
            <button
              type="button"
              class="shrink-0 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
              disabled={!quickTemplateId || props.locked}
              onclick={applyQuickTemplate}
            >
              应用
            </button>
          </div>

          <div class="mt-3 flex items-center gap-2">
            <input
              class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              placeholder="基于当前黑名单另存为新模板..."
              bind:value={saveTemplateName}
              disabled={draft.length === 0}
            />
            <button
              type="button"
              class="shrink-0 rounded-2xl bg-zinc-900 px-3 py-2 text-xs font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
              disabled={!saveTemplateName.trim() || draft.length === 0}
              onclick={() => void saveCurrentAsTemplate()}
            >
              另存为
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <BlacklistDraftList
            draft={draft}
            locked={props.locked}
            isOriginalName={isOriginalName}
            onDisplayNameChange={setDisplayName}
            onRemove={removeFromDraft}
          />
          <RunningProcessesPanel
            processes={processes}
            loading={loading}
            error={error}
            lastUpdatedAt={lastUpdatedAt}
            hasInDraft={hasInDraft}
            onToggle={toggleProcess}
            onRefresh={refreshProcessesNow}
          />
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
