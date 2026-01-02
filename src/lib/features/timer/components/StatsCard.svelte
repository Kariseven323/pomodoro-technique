<script lang="ts">
  import GoalProgress from "$lib/features/timer/GoalProgress.svelte";
  import TagPicker from "$lib/features/timer/components/TagPicker.svelte";
  import type { TimerSnapshot } from "$lib/shared/types";

  const props = $props<{
    snapshot: TimerSnapshot;
    tags: string[];
    onSelectTag: (tag: string) => void;
    onCreateTag: (tag: string) => void;
  }>();

  /** 转发标签选择事件到上层。 */
  function onSelect(e: CustomEvent<string>): void {
    props.onSelectTag(e.detail);
  }

  /** 转发标签创建事件到上层。 */
  function onCreate(e: CustomEvent<string>): void {
    props.onCreateTag(e.detail);
  }
</script>

<div
  class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60"
>
  <div class="mb-3 text-sm font-medium text-zinc-900 dark:text-zinc-50">目标进度</div>
  <GoalProgress progress={props.snapshot.goalProgress} />

  <div class="mt-6">
    <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">任务标签</div>
    <TagPicker
      tags={props.tags}
      currentTag={props.snapshot.currentTag}
      disabled={false}
      on:select={onSelect}
      on:create={onCreate}
    />
  </div>

  <div class="mt-6">
    <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">今日统计（按标签）</div>
    {#if props.snapshot.todayStats.byTag.length === 0}
      <div class="text-sm text-zinc-500 dark:text-zinc-400">今天还没有完成记录</div>
    {:else}
      <div class="space-y-2">
        {#each props.snapshot.todayStats.byTag as item (item.tag)}
          <div class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
            <div class="truncate">{item.tag}</div>
            <div class="tabular-nums">{item.count}</div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="mt-6 text-xs text-zinc-600 dark:text-zinc-300">
    黑名单状态：{props.snapshot.blacklistLocked ? "专注期锁定（仅可新增）" : "可编辑"}
  </div>
</div>
