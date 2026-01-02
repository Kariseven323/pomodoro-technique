<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const props = $props<{
    /** 标签列表。 */
    tags: string[];
    /** 当前选中的标签。 */
    currentTag: string;
    /** 是否禁用交互（例如快照未加载）。 */
    disabled: boolean;
  }>();

  const dispatch = createEventDispatcher<{
    /** 选择现有标签。 */
    select: string;
    /** 新建并选择标签。 */
    create: string;
  }>();

  let newTag = $state("");

  /** 处理下拉选择变化。 */
  function onSelectChange(e: Event): void {
    const el = e.currentTarget as HTMLSelectElement;
    const tag = el.value;
    if (!tag) return;
    dispatch("select", tag);
  }

  /** 提交新标签（并重置输入框）。 */
  function submitNewTag(): void {
    const tag = newTag.trim();
    if (!tag) return;
    newTag = "";
    dispatch("create", tag);
  }
</script>

<div class="flex flex-col gap-2">
  <select
    class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
    onchange={onSelectChange}
    value={props.currentTag}
    disabled={props.disabled}
  >
    {#each props.tags as t (t)}
      <option class="bg-white text-zinc-900" value={t}>{t}</option>
    {/each}
  </select>
  <div class="flex items-center gap-2">
    <input
      class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      placeholder="新增标签..."
      bind:value={newTag}
      disabled={props.disabled}
      onkeydown={(e) => {
        if (e.key === "Enter") submitNewTag();
      }}
    />
    <button
      class="shrink-0 rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
      onclick={submitNewTag}
      disabled={props.disabled || !newTag.trim()}
    >
      添加
    </button>
  </div>
</div>

