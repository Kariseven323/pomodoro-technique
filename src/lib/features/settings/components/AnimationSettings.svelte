<script lang="ts">
  import type { AnimationIntensity } from "$lib/shared/types";

  let {
    enabled = $bindable(),
    comboEnabled = $bindable(),
    intensity = $bindable(),
  } = $props<{
    enabled: boolean;
    comboEnabled: boolean;
    intensity: AnimationIntensity;
  }>();
</script>

<div class="mt-4 space-y-3">
  <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">完成动画</div>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">启用完成动画</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={enabled} />
  </label>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">启用 Combo 显示</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={comboEnabled} disabled={!enabled} />
  </label>

  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">动画强度</div>
    <select
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-zinc-100/90 dark:text-zinc-900"
      bind:value={intensity}
      disabled={!enabled}
    >
      <option value="minimal">简约</option>
      <option value="standard">标准</option>
      <option value="fancy">华丽</option>
    </select>
  </label>
</div>
