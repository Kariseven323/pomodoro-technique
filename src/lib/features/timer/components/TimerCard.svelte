<script lang="ts">
  import { phaseAccentClass, phaseLabel } from "$lib/utils/phase";
  import { formatMmSs } from "$lib/utils/time";
  import type { TimerSnapshot } from "$lib/shared/types";

  const props = $props<{
    snapshot: TimerSnapshot;
    requiresAdmin: boolean;
    onToggleStartPause: () => void;
    onReset: () => void;
    onSkip: () => void;
    onRestartAsAdmin: () => void;
  }>();
</script>

<div
  class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60"
>
  <div class="flex items-start justify-between">
    <div>
      <div class="text-sm text-zinc-600 dark:text-zinc-300">当前阶段</div>
      <div class={"mt-1 text-2xl font-semibold " + phaseAccentClass(props.snapshot.phase)}>
        {phaseLabel(props.snapshot.phase)}
      </div>
    </div>
    <div class="text-right">
      <div class="text-sm text-zinc-600 dark:text-zinc-300">今日完成</div>
      <div class="mt-1 text-xl font-semibold">{props.snapshot.todayStats.total}</div>
    </div>
  </div>

  <div class="mt-6 rounded-3xl bg-black/5 p-5 text-center dark:bg-white/10">
    <div class="text-5xl font-bold tabular-nums">{formatMmSs(props.snapshot.remainingSeconds)}</div>
    <div class="mt-2 text-sm text-zinc-600 dark:text-zinc-300">
      {props.snapshot.isRunning ? "计时中..." : "已暂停"}
    </div>
  </div>

  <div class="mt-5 grid grid-cols-3 gap-2">
    <button
      class="rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
      onclick={props.onToggleStartPause}
    >
      {props.snapshot.isRunning ? "暂停" : "开始"}
    </button>
    <button
      class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-800 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
      onclick={props.onReset}
    >
      重置
    </button>
    <button
      class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-800 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
      onclick={props.onSkip}
    >
      跳过
    </button>
  </div>

  {#if props.requiresAdmin}
    <div class="mt-4 rounded-2xl bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300">
      检测到部分进程需要管理员权限才能终止。
      <button class="ml-2 underline" onclick={props.onRestartAsAdmin}>以管理员身份重启</button>
    </div>
  {/if}
</div>
