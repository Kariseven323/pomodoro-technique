<script lang="ts">
  import type { BlacklistItem, BlacklistTemplate } from "$lib/shared/types";
  import { applyTemplate, deleteTemplate, getTemplates, saveTemplate } from "$lib/api/tauri";
  import { createEventDispatcher } from "svelte";
  import TemplateSelector from "$lib/features/blacklist/components/TemplateSelector.svelte";

  const props = $props<{
    open: boolean;
    locked: boolean;
    draft: BlacklistItem[];
    templates: BlacklistTemplate[];
    activeTemplateIds: string[];
  }>();

  const dispatch = createEventDispatcher<{
    templatesChange: { templates: BlacklistTemplate[]; activeTemplateIds: string[]; blacklist: BlacklistItem[] };
  }>();

  let templatesDraft = $state<BlacklistTemplate[]>([]);
  let activeTemplateIdsDraft = $state<string[]>([]);
  let templateNotice = $state<string | null>(null);
  let templateError = $state<string | null>(null);
  let saveTemplateName = $state("");
  let quickTemplateId = $state<string>("");

  /** 将 `templates/activeTemplateIds` 同步到本地草稿（用于模板切换与管理）。 */
  function syncFromProps(): void {
    templatesDraft = props.templates.map((t: BlacklistTemplate) => ({
      ...t,
      processes: t.processes.map((p: BlacklistItem) => ({ ...p })),
    }));
    activeTemplateIdsDraft = [...props.activeTemplateIds];
    quickTemplateId = templatesDraft[0]?.id ?? "";
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
      dispatch("templatesChange", {
        templates: templatesDraft,
        activeTemplateIds: activeTemplateIdsDraft,
        blacklist: props.draft,
      });
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
    }
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
      dispatch("templatesChange", {
        templates: templatesDraft,
        activeTemplateIds: nextActive,
        blacklist: nextBlacklist,
      });
      templateNotice = "已应用模板";
      window.setTimeout(clearTemplateNotice, 1800);
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
      const created = await saveTemplate({
        id: "",
        name,
        builtin: false,
        processes: props.draft.map((b: BlacklistItem) => ({ ...b })),
      });
      const next = templatesDraft.filter((t) => t.id !== created.id);
      next.push(created);
      templatesDraft = next;
      saveTemplateName = "";
      dispatch("templatesChange", {
        templates: templatesDraft,
        activeTemplateIds: activeTemplateIdsDraft,
        blacklist: props.draft,
      });
      templateNotice = "已保存为新模板";
      window.setTimeout(clearTemplateNotice, 1800);
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
      dispatch("templatesChange", {
        templates: templatesDraft,
        activeTemplateIds: activeTemplateIdsDraft,
        blacklist: props.draft,
      });
      templateNotice = "已删除模板";
      window.setTimeout(clearTemplateNotice, 1800);
    } catch (e) {
      templateError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 清理模板提示文案（用于定时隐藏）。 */
  function clearTemplateNotice(): void {
    templateNotice = null;
  }

  /** 响应 `open` 变化：打开时同步草稿并刷新模板，关闭时清理提示状态。 */
  function onOpenEffect(): void {
    if (props.open) {
      syncFromProps();
      void loadTemplates();
    } else {
      templateError = null;
      templateNotice = null;
    }
  }

  $effect(onOpenEffect);

  /** 转发“切换模板”事件到异步处理逻辑。 */
  function onToggle(e: CustomEvent<string>): void {
    void toggleTemplate(e.detail);
  }

  /** 转发“删除模板”事件到异步处理逻辑。 */
  function onDelete(e: CustomEvent<string>): void {
    void deleteTemplateById(e.detail);
  }

  /** 处理“另存为模板”按钮点击。 */
  function onSaveAsTemplateClick(): void {
    void saveCurrentAsTemplate();
  }
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
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
      on:toggle={onToggle}
      on:delete={onDelete}
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
      disabled={props.draft.length === 0}
    />
    <button
      type="button"
      class="shrink-0 rounded-2xl bg-zinc-900 px-3 py-2 text-xs font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
      disabled={!saveTemplateName.trim() || props.draft.length === 0}
      onclick={onSaveAsTemplateClick}
    >
      另存为
    </button>
  </div>
</div>
