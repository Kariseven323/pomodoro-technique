<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { BlacklistTemplate } from "$lib/types";

  const props = $props<{
    templates: BlacklistTemplate[];
    activeTemplateIds: string[];
    locked: boolean;
  }>();

  const dispatch = createEventDispatcher<{
    toggle: string;
  }>();

  /** 判断模板是否启用。 */
  function isActive(id: string): boolean {
    return props.activeTemplateIds.includes(id);
  }

  /** 切换模板启用状态。 */
  function onToggle(id: string): void {
    dispatch("toggle", id);
  }
</script>

<div class="space-y-2">
  {#each props.templates as t (t.id)}
    <label class="flex items-center justify-between gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
      <div class="flex min-w-0 items-center gap-2">
        <input class="h-4 w-4" type="checkbox" checked={isActive(t.id)} disabled={props.locked} onchange={() => onToggle(t.id)} />
        <div class="min-w-0">
          <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{t.name}</div>
          <div class="truncate text-[11px] text-zinc-500 dark:text-zinc-400">{t.processes.length} 个进程</div>
        </div>
      </div>
      {#if t.builtin}
        <span class="rounded-lg bg-zinc-900/10 px-2 py-0.5 text-[10px] text-zinc-700 dark:bg-white/10 dark:text-zinc-200">内置</span>
      {/if}
    </label>
  {/each}
</div>

