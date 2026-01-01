<script lang="ts">
  import type { GoalProgress } from "$lib/types";

  const props = $props<{ progress: GoalProgress }>();

  /** 计算百分比（0-100），目标为 0 时返回 0。 */
  function percent(completed: number, goal: number): number {
    if (goal <= 0) return 0;
    return Math.max(0, Math.min(100, Math.round((completed / goal) * 100)));
  }

  /** 计算超额完成数量（目标为 0 时返回 0）。 */
  function over(completed: number, goal: number): number {
    if (goal <= 0) return 0;
    return Math.max(0, completed - goal);
  }

  /** 计算进度条宽度（Tailwind 百分比字符串）。 */
  function widthStyle(completed: number, goal: number): string {
    return `width: ${percent(completed, goal)}%`;
  }
</script>

<div class="space-y-3">
  <div class="rounded-2xl border border-white/20 bg-white/70 p-4 shadow-sm backdrop-blur-xl dark:border-white/10 dark:bg-white/5">
    <div class="flex items-center justify-between text-sm">
      <div class="font-medium text-zinc-900 dark:text-zinc-50">今日目标</div>
      <div class="tabular-nums text-zinc-700 dark:text-zinc-200">
        {props.progress.dailyCompleted}/{props.progress.dailyGoal || "—"}
        {#if over(props.progress.dailyCompleted, props.progress.dailyGoal) > 0}
          <span class="ml-2 text-xs text-emerald-700 dark:text-emerald-300">
            +{over(props.progress.dailyCompleted, props.progress.dailyGoal)}
          </span>
        {/if}
      </div>
    </div>
    <div class="mt-2 h-2 overflow-hidden rounded-full bg-black/5 dark:bg-white/10">
      <div class="h-full rounded-full bg-emerald-500/80" style={widthStyle(props.progress.dailyCompleted, props.progress.dailyGoal)}></div>
    </div>
    {#if props.progress.dailyGoal > 0}
      <div class="mt-2 text-xs text-zinc-600 dark:text-zinc-300">{percent(props.progress.dailyCompleted, props.progress.dailyGoal)}%</div>
    {/if}
  </div>

  <div class="rounded-2xl border border-white/20 bg-white/70 p-4 shadow-sm backdrop-blur-xl dark:border-white/10 dark:bg-white/5">
    <div class="flex items-center justify-between text-sm">
      <div class="font-medium text-zinc-900 dark:text-zinc-50">本周目标</div>
      <div class="tabular-nums text-zinc-700 dark:text-zinc-200">
        {props.progress.weeklyCompleted}/{props.progress.weeklyGoal || "—"}
        {#if over(props.progress.weeklyCompleted, props.progress.weeklyGoal) > 0}
          <span class="ml-2 text-xs text-emerald-700 dark:text-emerald-300">
            +{over(props.progress.weeklyCompleted, props.progress.weeklyGoal)}
          </span>
        {/if}
      </div>
    </div>
    <div class="mt-2 h-2 overflow-hidden rounded-full bg-black/5 dark:bg-white/10">
      <div class="h-full rounded-full bg-sky-500/80" style={widthStyle(props.progress.weeklyCompleted, props.progress.weeklyGoal)}></div>
    </div>
    {#if props.progress.weeklyGoal > 0}
      <div class="mt-2 text-xs text-zinc-600 dark:text-zinc-300">{percent(props.progress.weeklyCompleted, props.progress.weeklyGoal)}%</div>
    {/if}
  </div>
</div>

