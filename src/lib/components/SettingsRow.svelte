<script lang="ts">
  import type { Snippet } from "svelte";

  const props = $props<{
    /** 左侧标题。 */
    title: string;
    /** 右侧辅助文本（可选）。 */
    value?: string;
    /** 是否展示右侧箭头（用于可点击行）。 */
    chevron?: boolean;
    /** 默认插槽内容（Svelte 5 `children`）。 */
    children?: Snippet;
  }>();
</script>

<div
  class="flex items-center justify-between gap-3 border-b border-black/5 px-4 py-3 last:border-b-0 dark:border-white/10"
>
  <div class="min-w-0 text-sm text-zinc-900 dark:text-zinc-50">{props.title}</div>
  <div class="flex items-center gap-2">
    {#if props.value}
      <div class="max-w-[180px] truncate text-sm text-zinc-500 dark:text-zinc-400">{props.value}</div>
    {/if}
    {@render props.children?.()}
    {#if props.chevron}
      <div class="text-zinc-400 dark:text-zinc-500" aria-hidden="true">›</div>
    {/if}
  </div>
</div>
