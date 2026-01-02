<script lang="ts">
  import { dev } from "$app/environment";
  import DebugSection from "$lib/features/settings/DebugSection.svelte";
  import type { Settings } from "$lib/shared/types";
  import { createEventDispatcher } from "svelte";
  import TimerDurationsSection from "$lib/features/settings/components/TimerDurationsSection.svelte";
  import AutoContinueSection from "$lib/features/settings/components/AutoContinueSection.svelte";
  import GoalsSection from "$lib/features/settings/components/GoalsSection.svelte";
  import StorePathsSection from "$lib/features/settings/components/StorePathsSection.svelte";

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
    autoContinueEnabled: false,
    autoContinuePomodoros: 4,
    dailyGoal: 8,
    weeklyGoal: 40,
    alwaysOnTop: false,
  });

  let wasOpen = $state(false);

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

  /** 响应 `open` 变化：仅在“从关闭到打开”的瞬间同步草稿，避免编辑中被外部刷新覆盖。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      syncDraftFromProps();
    }
    wasOpen = props.open;
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
        class="flex max-h-[85vh] w-full max-w-md flex-col rounded-3xl border border-white/20 bg-white/80 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/70"
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

        <div class="min-h-0 flex-1 overflow-y-auto pr-1">
          <TimerDurationsSection
            bind:pomodoro={draft.pomodoro}
            bind:shortBreak={draft.shortBreak}
            bind:longBreak={draft.longBreak}
            bind:longBreakInterval={draft.longBreakInterval}
          />
          <AutoContinueSection
            bind:autoContinueEnabled={draft.autoContinueEnabled}
            bind:autoContinuePomodoros={draft.autoContinuePomodoros}
          />
          <GoalsSection
            bind:dailyGoal={draft.dailyGoal}
            bind:weeklyGoal={draft.weeklyGoal}
            bind:alwaysOnTop={draft.alwaysOnTop}
          />

          <StorePathsSection open={props.open} />

          {#if dev}
            <div class="mt-4">
              <DebugSection />
            </div>
          {/if}
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
