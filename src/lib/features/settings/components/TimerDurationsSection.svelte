<script lang="ts">
  let {
    pomodoro = $bindable(),
    shortBreak = $bindable(),
    longBreak = $bindable(),
    longBreakInterval = $bindable(),
  } = $props<{
    pomodoro: number;
    shortBreak: number;
    longBreak: number;
    longBreakInterval: number;
  }>();
</script>

<div class="space-y-3">
  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">番茄时长（1-60 分钟）</div>
    <input
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      type="number"
      min="1"
      max="60"
      bind:value={pomodoro}
    />
  </label>

  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">短休息（1-30 分钟）</div>
    <input
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      type="number"
      min="1"
      max="30"
      bind:value={shortBreak}
    />
  </label>

  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">长休息（1-60 分钟）</div>
    <input
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      type="number"
      min="1"
      max="60"
      bind:value={longBreak}
    />
  </label>

  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">长休息间隔（1-10 个番茄）</div>
    <input
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 ring-0 outline-none focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
      type="number"
      min="1"
      max="10"
      bind:value={longBreakInterval}
    />
  </label>
</div>
