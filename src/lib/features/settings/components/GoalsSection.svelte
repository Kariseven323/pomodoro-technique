<script lang="ts">
  let {
    dailyGoal = $bindable(),
    weeklyGoal = $bindable(),
    alwaysOnTop = $bindable(),
  } = $props<{
    dailyGoal: number;
    weeklyGoal: number;
    alwaysOnTop: boolean;
  }>();
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
  <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">目标设定</div>
  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    <label class="block">
      <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">每日目标（0 表示不设）</div>
      <input
        class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
        type="number"
        min="0"
        max="1000"
        bind:value={dailyGoal}
      />
    </label>
    <label class="block">
      <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">每周目标（0 表示不设）</div>
      <input
        class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
        type="number"
        min="0"
        max="10000"
        bind:value={weeklyGoal}
      />
    </label>
  </div>
  <label class="mt-3 flex items-center justify-between gap-3">
    <div class="text-sm text-zinc-700 dark:text-zinc-200">窗口置顶（主窗口）</div>
    <input class="h-4 w-4" type="checkbox" bind:checked={alwaysOnTop} />
  </label>
</div>
