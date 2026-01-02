<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { formatMmSs } from "$lib/utils/time";

  type InterruptionAction = "reset" | "skip";

  const props = $props<{
    open: boolean;
    action: InterruptionAction;
    tag: string;
    focusedSeconds: bigint;
    remainingSeconds: bigint;
  }>();

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: { record: boolean; reason: string };
  }>();

  const presets: string[] = ["紧急事务", "电话/消息", "会议", "休息需求", "其他"];

  let recordEnabled = $state(true);
  let selectedPreset = $state<string>(presets[0] ?? "其他");
  let customReason = $state<string>("");

  /** 关闭弹窗（不执行操作）。 */
  function close(): void {
    dispatch("close");
  }

  /** 生成最终原因文案（允许为空）。 */
  function buildReason(): string {
    if (!recordEnabled) return "";
    if (selectedPreset === "其他") return customReason.trim();
    return selectedPreset;
  }

  /** 确认并继续：将“是否记录/原因”回传给上层。 */
  function confirm(): void {
    dispatch("confirm", { record: recordEnabled, reason: buildReason() });
  }

  /** 在打开时重置交互状态，避免上一次输入残留。 */
  function onOpenEffect(): void {
    if (!props.open) return;
    recordEnabled = true;
    selectedPreset = presets[0] ?? "其他";
    customReason = "";
  }

  $effect(onOpenEffect);
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button type="button" class="absolute inset-0 bg-black/30 backdrop-blur-sm" aria-label="关闭弹窗" onclick={close}
    ></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div
        class="w-full max-w-md rounded-3xl border border-white/20 bg-white/85 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/75"
      >
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-50">番茄中断确认</h2>
          <button
            class="rounded-xl px-3 py-1 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={close}
          >
            关闭
          </button>
        </div>

        <div class="space-y-3 text-sm text-zinc-700 dark:text-zinc-200">
          <div class="rounded-2xl bg-black/5 p-3 dark:bg-white/10">
            <div>当前标签：<span class="font-medium text-zinc-900 dark:text-zinc-50">{props.tag}</span></div>
            <div class="mt-1">
              已专注：<span class="font-medium tabular-nums">{formatMmSs(props.focusedSeconds)}</span>
              · 剩余：<span class="font-medium tabular-nums">{formatMmSs(props.remainingSeconds)}</span>
            </div>
          </div>

          <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 dark:bg-white/10">
            <div class="text-sm">记录本次中断</div>
            <input type="checkbox" class="h-4 w-4" bind:checked={recordEnabled} />
          </label>

          <div class={"space-y-2 " + (recordEnabled ? "" : "opacity-50")}>
            <div class="text-xs text-zinc-600 dark:text-zinc-300">中断原因（可选）</div>
            <select
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-60 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              bind:value={selectedPreset}
              disabled={!recordEnabled}
            >
              {#each presets as p (p)}
                <option value={p}>{p}</option>
              {/each}
            </select>

            {#if selectedPreset === "其他"}
              <input
                class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-60 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                placeholder="可输入自定义原因（可留空）"
                bind:value={customReason}
                disabled={!recordEnabled}
              />
            {/if}
          </div>
        </div>

        <div class="mt-5 grid grid-cols-2 gap-2">
          <button
            class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-700 hover:bg-white dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
            onclick={close}
          >
            取消
          </button>
          <button
            class="rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
            onclick={confirm}
          >
            {props.action === "reset" ? "确认重置" : "确认跳过"}
          </button>
        </div>

        <div class="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
          提示：点击“确认”后将继续执行 {props.action === "reset" ? "重置" : "跳过"} 操作。
        </div>
      </div>
    </div>
  </div>
{/if}
