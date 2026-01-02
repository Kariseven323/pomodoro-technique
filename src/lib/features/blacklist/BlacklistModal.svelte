<script lang="ts">
  import type { BlacklistItem, BlacklistTemplate, ProcessInfo } from "$lib/shared/types";
  import { listProcesses } from "$lib/api/tauri";
  import { createEventDispatcher } from "svelte";
  import BlacklistDraftList from "$lib/features/blacklist/components/BlacklistDraftList.svelte";
  import RunningProcessesPanel from "$lib/features/blacklist/components/RunningProcessesPanel.svelte";
  import TemplateManager from "$lib/features/blacklist/components/TemplateManager.svelte";

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

  let initialLoading = $state(false);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let processes = $state<ProcessInfo[]>([]);
  let lastUpdatedAt = $state<number | null>(null);
  let draft = $state<BlacklistItem[]>([]);
  let originalNames = $state<string[]>([]);
  let lastOpen = false;

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

  /** 从后端加载进程列表（避免并发请求导致 UI 抖动，同时避免刷新时“整块列表闪烁/不可操作”）。 */
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

  /** 立即刷新进程列表（用于按钮点击/聚焦回到窗口时）。 */
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

  /** 响应 `open` 的“边沿变化”：仅在打开/关闭瞬间做初始化与清理，避免重复触发导致“实时刷新”错觉。 */
  function onOpenEdgeEffect(): void {
    const isOpen = props.open;
    if (isOpen === lastOpen) return;
    lastOpen = isOpen;
    if (isOpen) {
      syncDraftFromProps();
      // 延迟到下一轮事件循环再触发拉取，确保弹窗优先渲染出来，避免“点击后无界面”的卡顿感知。
      window.setTimeout(refreshProcessesNow, 0);
      return;
    }
    error = null;
  }

  $effect(onOpenEdgeEffect);

  /** 判断某条目是否为打开弹窗时“已有条目”（用于锁定时禁用移除）。 */
  function isOriginalName(name: string): boolean {
    return originalNames.includes(normalizeName(name));
  }

  /** 处理模板管理器推送的模板变更：同步黑名单草稿并向上抛出事件。 */
  function onTemplatesChange(
    e: CustomEvent<{ templates: BlacklistTemplate[]; activeTemplateIds: string[]; blacklist: BlacklistItem[] }>,
  ): void {
    draft = e.detail.blacklist.map((b) => ({ ...b }));
    originalNames = draft.map((b) => normalizeName(b.name));
    dispatch("templatesChange", e.detail);
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
        class="flex max-h-[85vh] w-full max-w-2xl flex-col rounded-3xl border border-white/20 bg-white/80 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/70"
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

        <div class="min-h-0 flex-1 overflow-y-auto pr-1">
          <TemplateManager
            open={props.open}
            locked={props.locked}
            templates={props.templates}
            activeTemplateIds={props.activeTemplateIds}
            {draft}
            on:templatesChange={onTemplatesChange}
          />

          <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
            <BlacklistDraftList
              {draft}
              locked={props.locked}
              {isOriginalName}
              onDisplayNameChange={setDisplayName}
              onRemove={removeFromDraft}
            />
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
