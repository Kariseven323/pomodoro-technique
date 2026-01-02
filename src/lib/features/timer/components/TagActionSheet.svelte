<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const props = $props<{
    /** 是否显示 action sheet。 */
    open: boolean;
    /** 标签列表（用于选择/搜索）。 */
    tags: string[];
    /** 当前选中的标签。 */
    currentTag: string;
  }>();

  const dispatch = createEventDispatcher<{
    close: void;
    select: string;
    create: string;
    manage: void;
  }>();

  let wasOpen = $state(false);
  let query = $state("");
  let createDraft = $state("");

  /** 关闭 action sheet（不做任何操作）。 */
  function close(): void {
    dispatch("close");
  }

  /** 点击某个标签：触发选择并关闭。 */
  function selectTag(tag: string): void {
    const next = tag.trim();
    if (!next) return;
    dispatch("select", next);
  }

  /** 提交创建：触发创建并关闭。 */
  function createTag(): void {
    const next = createDraft.trim();
    if (!next) return;
    dispatch("create", next);
  }

  /** 打开“管理标签”。 */
  function manageTags(): void {
    dispatch("manage");
  }

  /** 响应 open：打开时重置搜索/输入，避免上次状态残留。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      query = "";
      createDraft = "";
    }
    wasOpen = props.open;
  }

  $effect(onOpenEffect);

  /** 将标签列表按 query 进行过滤（大小写不敏感）。 */
  function filteredTags(tags: string[], q: string): string[] {
    const needle = q.trim().toLowerCase();
    if (!needle) return tags;
    return tags.filter((t) => t.toLowerCase().includes(needle));
  }

  const visibleTags = $derived(filteredTags(props.tags, query));
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button type="button" class="absolute inset-0 bg-black/30" aria-label="关闭标签选择" onclick={close}></button>
    <div class="absolute inset-x-0 bottom-0 p-4">
      <div class="mx-auto w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-sm dark:bg-zinc-900">
        <div class="border-b border-black/5 p-4 dark:border-white/10">
          <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">选择标签</div>
          <div class="mt-2 flex items-center gap-2">
            <input
              class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-50"
              placeholder="搜索标签"
              bind:value={query}
            />
            <button
              type="button"
              class="shrink-0 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={manageTags}
            >
              管理
            </button>
          </div>

          <div class="mt-3 flex items-center gap-2">
            <input
              class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-50"
              placeholder="新增标签（回车创建）"
              bind:value={createDraft}
              onkeydown={(e) => {
                if (e.key === "Enter") createTag();
              }}
            />
            <button
              type="button"
              class="shrink-0 rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow-sm hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
              onclick={createTag}
              disabled={!createDraft.trim()}
            >
              新增
            </button>
          </div>
        </div>

        <div class="max-h-[55vh] overflow-y-auto p-2">
          {#if visibleTags.length === 0}
            <div class="p-3 text-sm text-zinc-500 dark:text-zinc-400">没有匹配的标签</div>
          {:else}
            {#each visibleTags as t (t)}
              <button
                type="button"
                class={"flex w-full items-center justify-between rounded-2xl px-4 py-3 text-left text-sm " +
                  (t === props.currentTag
                    ? "bg-black/5 text-zinc-900 dark:bg-white/10 dark:text-zinc-50"
                    : "text-zinc-800 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10")}
                onclick={() => selectTag(t)}
              >
                <span class="truncate">{t}</span>
                {#if t === props.currentTag}
                  <span class="text-xs text-zinc-500 dark:text-zinc-400">当前</span>
                {/if}
              </button>
            {/each}
          {/if}
        </div>

        <div class="border-t border-black/5 p-2 dark:border-white/10">
          <button
            type="button"
            class="w-full rounded-2xl px-4 py-3 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={close}
          >
            取消
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
