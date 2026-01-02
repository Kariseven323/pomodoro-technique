<script lang="ts">
  import { dev } from "$app/environment";
  import DebugSection from "$lib/components/DebugSection.svelte";
  import type { Settings } from "$lib/types";
  import type { StorePaths } from "$lib/types";
  import { getStorePaths, openLogDir as openLogDirApi, openStoreDir as openStoreDirApi } from "$lib/tauriApi";
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
    autoContinueEnabled: false,
    autoContinuePomodoros: 4,
    dailyGoal: 8,
    weeklyGoal: 40,
    alwaysOnTop: false,
  });

  let storePaths = $state<StorePaths | null>(null);
  let storePathsLoading = $state(false);
  let storePathsError = $state<string | null>(null);
  let storePathsNotice = $state<string | null>(null);
  let logNotice = $state<string | null>(null);
  let logError = $state<string | null>(null);
  let wasOpen = $state(false);

  /** 将外部传入的 `settings` 同步到本地草稿（用于编辑）。 */
  function syncDraftFromProps(): void {
    draft = { ...props.settings };
  }

  /** 加载应用数据根目录路径（统一入口，用于设置页展示）。 */
  async function loadStorePaths(): Promise<void> {
    if (storePathsLoading) return;
    storePathsLoading = true;
    storePathsError = null;
    try {
      storePaths = await getStorePaths();
    } catch (e) {
      storePathsError = e instanceof Error ? e.message : String(e);
      storePaths = null;
    } finally {
      storePathsLoading = false;
    }
  }

  /** 点击“打开文件夹”时打开应用数据根目录（统一入口）。 */
  async function onOpenStoreDirClick(): Promise<void> {
    storePathsError = null;
    storePathsNotice = null;
    if (!storePaths) await loadStorePaths();
    try {
      await openStoreDirApi();
      storePathsNotice = "已请求打开文件夹";
      window.setTimeout((): void => {
        storePathsNotice = null;
      }, 2000);
    } catch (e) {
      storePathsError = e instanceof Error ? e.message : String(e);
    }
  }

  /** 点击“查看日志”时打开日志目录（文件管理器）。 */
  async function onOpenLogDirClick(): Promise<void> {
    logError = null;
    logNotice = null;
    try {
      await openLogDirApi();
      logNotice = "已请求打开日志目录";
      window.setTimeout((): void => {
        logNotice = null;
      }, 2000);
    } catch (e) {
      logError = e instanceof Error ? e.message : String(e);
    }
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
      void loadStorePaths();
    }
    if (!props.open && wasOpen) {
      storePathsError = null;
      storePathsNotice = null;
      logError = null;
      logNotice = null;
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

            <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
              <label class="flex items-center justify-between gap-3">
                <div class="text-sm text-zinc-700 dark:text-zinc-200">休息结束后自动进入工作倒计时</div>
                <input class="h-4 w-4" type="checkbox" bind:checked={draft.autoContinueEnabled} />
              </label>
              <label class="mt-3 block">
                <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">连续番茄数量（1-20 个番茄）</div>
                <input
                  class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                  type="number"
                  min="1"
                  max="20"
                  disabled={!draft.autoContinueEnabled}
                  bind:value={draft.autoContinuePomodoros}
                />
              </label>
            </div>

            <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
              <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">目标设定</div>
              <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <label class="block">
                  <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">每日目标（0 表示不设）</div>
                  <input
                    class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                    type="number"
                    min="0"
                    max="1000"
                    bind:value={draft.dailyGoal}
                  />
                </label>
                <label class="block">
                  <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">每周目标（0 表示不设）</div>
                  <input
                    class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-zinc-900 outline-none ring-0 focus:border-black/20 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                    type="number"
                    min="0"
                    max="10000"
                    bind:value={draft.weeklyGoal}
                  />
                </label>
              </div>
              <label class="mt-3 flex items-center justify-between gap-3">
                <div class="text-sm text-zinc-700 dark:text-zinc-200">窗口置顶（主窗口）</div>
                <input class="h-4 w-4" type="checkbox" bind:checked={draft.alwaysOnTop} />
              </label>
            </div>
          </div>

          <div class="mt-4 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">数据根目录（统一入口）</div>
            {#if storePathsNotice}
              <div class="mb-2 text-xs text-emerald-700 dark:text-emerald-300">{storePathsNotice}</div>
            {/if}
            {#if storePathsError}
              <div class="mb-2 text-xs text-red-600 dark:text-red-300">失败：{storePathsError}</div>
            {/if}
            <div class="flex items-center gap-2">
              <input
                class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                readonly
                value={storePaths?.storeDirPath ?? ""}
                placeholder={storePathsLoading ? "正在获取..." : "未获取到路径"}
              />
              <button
                type="button"
                class="shrink-0 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
                disabled={storePathsLoading}
                onclick={onOpenStoreDirClick}
              >
                打开文件夹
              </button>
            </div>
          </div>

          <div class="mt-4 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">日志</div>
            {#if logNotice}
              <div class="mb-2 text-xs text-emerald-700 dark:text-emerald-300">{logNotice}</div>
            {/if}
            {#if logError}
              <div class="mb-2 text-xs text-red-600 dark:text-red-300">失败：{logError}</div>
            {/if}
            <button
              type="button"
              class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
              onclick={onOpenLogDirClick}
            >
              查看日志
            </button>
          </div>

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
