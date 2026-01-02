<script lang="ts">
  import type { BlacklistItem } from "$lib/shared/types";

  const props = $props<{
    /** 当前草稿黑名单。 */
    draft: BlacklistItem[];
    /** 是否处于专注期锁定（锁定时禁止移除原有条目）。 */
    locked: boolean;
    /** 判断某个条目是否为“原有条目”（用于锁定时禁用移除）。 */
    isOriginalName: (name: string) => boolean;
    /** 更新显示名（由上层维护草稿）。 */
    onDisplayNameChange: (name: string, displayName: string) => void;
    /** 请求移除条目（由上层决定是否允许）。 */
    onRemove: (name: string) => void;
  }>();

  /** 处理显示名输入变化。 */
  function onDisplayNameInput(name: string, e: Event): void {
    const el = e.currentTarget as HTMLInputElement;
    props.onDisplayNameChange(name, el.value);
  }

  /** 处理移除按钮点击。 */
  function onRemoveClick(name: string): void {
    props.onRemove(name);
  }
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-4 dark:border-white/10 dark:bg-white/5">
  <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">已加入黑名单</div>
  {#if props.draft.length === 0}
    <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无黑名单</div>
  {:else}
    <div class="max-h-80 space-y-2 overflow-auto pr-1">
      {#each props.draft as item (item.name)}
        <div class="flex items-center gap-2 rounded-2xl bg-black/5 p-2 dark:bg-white/10">
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm text-zinc-900 dark:text-zinc-50">{item.name}</div>
            <input
              class="mt-1 w-full rounded-xl border border-black/10 bg-white/70 px-2 py-1 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              value={item.displayName}
              oninput={(e) => onDisplayNameInput(item.name, e)}
              placeholder="显示名"
            />
          </div>
          <button
            class="rounded-xl px-2 py-1 text-xs text-zinc-600 hover:bg-black/10 disabled:opacity-40 dark:text-zinc-200 dark:hover:bg-white/10"
            disabled={props.locked && props.isOriginalName(item.name)}
            onclick={() => onRemoveClick(item.name)}
          >
            移除
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

