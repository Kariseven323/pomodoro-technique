<script lang="ts">
  let { autoContinueEnabled = $bindable(), autoContinuePomodoros = $bindable() } = $props<{
    autoContinueEnabled: boolean;
    autoContinuePomodoros: number;
  }>();
</script>

<div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
  <label class="flex items-center justify-between gap-3">
    <div class="text-sm text-zinc-700 dark:text-zinc-200">休息结束后自动进入工作倒计时</div>
    <input class="h-4 w-4" type="checkbox" bind:checked={autoContinueEnabled} />
  </label>
  <label class="mt-3 block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">连续番茄数量（1-20 个番茄）</div>
    <input
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      type="number"
      min="1"
      max="20"
      disabled={!autoContinueEnabled}
      bind:value={autoContinuePomodoros}
    />
  </label>
</div>
