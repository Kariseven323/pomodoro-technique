<script lang="ts">
  import type { Snippet } from "svelte";

  const props = $props<{
    /** 分组标题（可选）。 */
    title?: string;
    /** 默认插槽内容（Svelte 5 `children`）。 */
    children?: Snippet;
  }>();
</script>

<section class="mt-6">
  {#if props.title}
    <div class="mb-2 px-1 text-xs font-medium text-zinc-500 dark:text-zinc-400">{props.title}</div>
  {/if}
  <div class="overflow-hidden rounded-2xl bg-white shadow-sm dark:bg-zinc-900">
    {@render props.children?.()}
  </div>
</section>
