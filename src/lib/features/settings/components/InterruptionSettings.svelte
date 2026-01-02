<script lang="ts">
  let { enabled = $bindable(), confirmOnInterrupt = $bindable() } = $props<{
    enabled: boolean;
    confirmOnInterrupt: boolean;
  }>();
</script>

<div class="mt-4 space-y-3">
  <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">中断记录</div>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">记录中断</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={enabled} />
  </label>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">中断时弹窗确认</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={confirmOnInterrupt} disabled={!enabled} />
  </label>
</div>
