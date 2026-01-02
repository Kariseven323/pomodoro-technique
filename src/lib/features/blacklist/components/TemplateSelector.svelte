<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { BlacklistTemplate } from "$lib/shared/types";

  const props = $props<{
    templates: BlacklistTemplate[];
    activeTemplateIds: string[];
    locked: boolean;
  }>();

  const dispatch = createEventDispatcher<{
    toggle: string;
    delete: string;
  }>();

  /** 判断模板是否启用。 */
  function isActive(id: string): boolean {
    return props.activeTemplateIds.includes(id);
  }

  /** 切换模板启用状态。 */
  function onToggle(id: string): void {
    dispatch("toggle", id);
  }

  /** 请求删除模板（仅用于自定义模板；由上层决定是否允许）。 */
  function onDelete(id: string): void {
    dispatch("delete", id);
  }
</script>

<div class="space-y-2">
  {#each props.templates as t (t.id)}
    <div class="flex items-center justify-between gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
      <label class="flex min-w-0 items-center gap-2">
        <input class="h-4 w-4" type="checkbox" checked={isActive(t.id)} disabled={props.locked} onchange={() => onToggle(t.id)} />
        <div class="min-w-0">
          <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{t.name}</div>
          <div class="truncate text-[11px] text-zinc-500 dark:text-zinc-400">{t.processes.length} 个进程</div>
        </div>
      </label>
      <div class="flex items-center gap-2">
        {#if t.builtin}
          <span class="rounded-lg bg-zinc-900/10 px-2 py-0.5 text-[10px] text-zinc-700 dark:bg-white/10 dark:text-zinc-200">内置</span>
        {:else}
          <button
            type="button"
            class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/10 disabled:opacity-40 dark:text-zinc-200 dark:hover:bg-white/10"
            disabled={props.locked}
            onclick={() => onDelete(t.id)}
          >
            删除
          </button>
        {/if}
      </div>
    </div>
  {/each}
</div>
