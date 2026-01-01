<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { WorkCompletedEvent } from "$lib/types";

  const props = $props<{ open: boolean; event: WorkCompletedEvent | null }>();

  const dispatch = createEventDispatcher<{
    close: void;
    save: string;
  }>();

  let remark = $state("");
  let wasOpen = $state(false);

  /** 响应 open 变化：打开时初始化备注草稿。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      remark = props.event?.record.remark ?? "";
    }
    wasOpen = props.open;
  }

  $effect(onOpenEffect);

  /** 关闭弹窗（不保存）。 */
  function closeModal(): void {
    dispatch("close");
  }

  /** 保存备注并关闭。 */
  function saveAndClose(): void {
    dispatch("save", remark.trim());
  }
</script>

{#if props.open && props.event}
  <div class="fixed inset-0 z-50">
    <button
      type="button"
      class="absolute inset-0 bg-black/30 backdrop-blur-sm"
      aria-label="关闭弹窗"
      onclick={closeModal}
    ></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div
        class="w-full max-w-md rounded-3xl border border-white/20 bg-white/80 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/70"
      >
        <div class="mb-3 flex items-start justify-between gap-3">
          <div>
            <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-50">番茄完成</h2>
            <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">
              {props.event.date} · {props.event.record.startTime} · {props.event.record.tag}
            </p>
          </div>
          <button
            class="rounded-xl px-3 py-1 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            关闭
          </button>
        </div>

        <label class="block">
          <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">备注（可选）</div>
          <textarea
            class="h-28 w-full resize-none rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
            placeholder="这一个番茄你做了些什么？"
            bind:value={remark}
          ></textarea>
        </label>

        <div class="mt-4 flex items-center justify-end gap-2">
          <button
            class="rounded-2xl px-4 py-2 text-sm text-zinc-700 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            跳过
          </button>
          <button
            class="rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
            onclick={saveAndClose}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

