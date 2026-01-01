<script lang="ts">
  import type { Settings } from "$lib/types";
  import { createEventDispatcher } from "svelte";

  const props = $props<{ open: boolean; settings: Settings }>();

  const dispatch = createEventDispatcher<{
    close: void;
    save: Settings;
  }>();

  let draft = $state<Settings>({
    pomodoro: 25,
    shortBreak: 5,
    longBreak: 15,
    longBreakInterval: 4,
  });

  /** 将外部传入的 `settings` 同步到本地草稿（用于编辑）。 */
  function syncDraftFromProps(): void {
    draft = { ...props.settings };
  }

  /** 关闭弹窗（不保存）。 */
  function closeModal(): void {
    dispatch("close");
  }

  /** 保存草稿并关闭弹窗。 */
  function saveAndClose(): void {
    dispatch("save", { ...draft });
  }

  /** 响应 `open` 变化：打开时同步草稿。 */
  function onOpenEffect(): void {
    if (props.open) {
      syncDraftFromProps();
    }
  }

  $effect(onOpenEffect);
</script>

{#if props.open}
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
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-50">设置</h2>
          <button
            class="rounded-xl px-3 py-1 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            关闭
          </button>
        </div>

        <div class="space-y-3">
          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">番茄时长（1-60 分钟）</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="number"
              min="1"
              max="60"
              bind:value={draft.pomodoro}
            />
          </label>

          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">短休息（1-30 分钟）</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="number"
              min="1"
              max="30"
              bind:value={draft.shortBreak}
            />
          </label>

          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">长休息（1-60 分钟）</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="number"
              min="1"
              max="60"
              bind:value={draft.longBreak}
            />
          </label>

          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">长休息间隔（1-10 个番茄）</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="number"
              min="1"
              max="10"
              bind:value={draft.longBreakInterval}
            />
          </label>
        </div>

        <div class="mt-5 flex items-center justify-end gap-2">
          <button
            class="rounded-2xl px-4 py-2 text-sm text-zinc-700 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            取消
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
