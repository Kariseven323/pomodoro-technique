<script lang="ts">
  import SettingsModal from "$lib/features/settings/SettingsModal.svelte";
  import BlacklistModal from "$lib/features/blacklist/BlacklistModal.svelte";
  import RemarkModal from "$lib/features/timer/RemarkModal.svelte";
  import MoreMenu from "$lib/features/timer/components/MoreMenu.svelte";
  import TimerCard from "$lib/features/timer/components/TimerCard.svelte";
  import StatsCard from "$lib/features/timer/components/StatsCard.svelte";
  import { appData, appError, appLoading, killSummary, timerSnapshot, workCompleted } from "$lib/stores/appClient";
  import { miniMode } from "$lib/stores/uiState";
  import { isTauri } from "@tauri-apps/api/core";
  import { restartAsAdmin, setHistoryRemark, setMiniMode } from "$lib/api/tauri";
  import type { AppData, Settings, WorkCompletedEvent } from "$lib/shared/types";
  import { useBlacklist } from "$lib/composables/useBlacklist";
  import { useSettings } from "$lib/composables/useSettings";
  import { useTags } from "$lib/composables/useTags";
  import { useTimer } from "$lib/composables/useTimer";
  import { useToast } from "$lib/composables/useToast";

  let settingsOpen = $state(false);
  let blacklistOpen = $state(false);
  let remarkOpen = $state(false);
  let remarkEvent = $state<WorkCompletedEvent | null>(null);
  const { toast: toastMessage, showToast } = useToast();

  const timer = useTimer({ showToast });
  const settings = useSettings({ showToast });
  const tags = useTags({ showToast });
  const blacklist = useBlacklist({ showToast });

  /** 在快照尚未加载时，为设置弹窗提供一个可用的默认值。 */
  const fallbackSettings: Settings = {
    pomodoro: 25,
    shortBreak: 5,
    longBreak: 15,
    longBreakInterval: 4,
    autoContinueEnabled: false,
    autoContinuePomodoros: 4,
    dailyGoal: 8,
    weeklyGoal: 40,
    alwaysOnTop: false,
  };

  /** 判断当前是否运行在 Tauri 宿主环境（非纯浏览器 dev server）。 */
  function isTauriRuntime(): boolean {
    try {
      return isTauri();
    } catch {
      return false;
    }
  }

  /** 打开设置弹窗。 */
  function handleOpenSettings(): void {
    settingsOpen = true;
  }

  /** 关闭设置弹窗。 */
  function handleCloseSettings(): void {
    settingsOpen = false;
  }

  /** 打开黑名单弹窗。 */
  function handleOpenBlacklist(): void {
    if (!isTauriRuntime()) {
      showToast("“管理黑名单”仅桌面端可用，请使用 `bun run tauri dev` 启动。");
      return;
    }
    blacklistOpen = true;
  }

  /** 关闭黑名单弹窗。 */
  function handleCloseBlacklist(): void {
    blacklistOpen = false;
  }

  /** 请求以管理员身份重启（用于终止需要提权的进程）。 */
  async function handleRestartAsAdmin(): Promise<void> {
    try {
      await restartAsAdmin();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 切换迷你模式（用于右上角菜单与托盘入口）。 */
  async function handleToggleMiniMode(): Promise<void> {
    const next = !$miniMode;
    try {
      await setMiniMode(next);
      miniMode.set(next);
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 打开备注弹窗。 */
  function openRemarkModal(e: WorkCompletedEvent): void {
    remarkEvent = e;
    remarkOpen = true;
  }

  /** 关闭备注弹窗。 */
  function closeRemarkModal(): void {
    remarkOpen = false;
    remarkEvent = null;
    workCompleted.set(null);
  }

  /** 保存备注（写入后端并关闭弹窗）。 */
  async function saveRemark(remark: string): Promise<void> {
    if (!remarkEvent) return;
    try {
      await setHistoryRemark(remarkEvent.date, remarkEvent.recordIndex, remark);
      closeRemarkModal();
      showToast("已保存备注");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 监听“工作完成事件”：自动弹出备注窗口。 */
  function onWorkCompletedEffect(): void {
    if ($workCompleted && !remarkOpen) {
      openRemarkModal($workCompleted);
    }
  }

  $effect(onWorkCompletedEffect);

  /** 当后端提示需要提权时，展示操作入口。 */
  function onKillSummaryEffect(): void {
    if ($killSummary?.requiresAdmin) {
      showToast("部分进程需要管理员权限才能终止");
    }
  }

  $effect(onKillSummaryEffect);

  /** 处理设置弹窗的保存事件。 */
  async function handleSettingsSave(e: CustomEvent<Settings>): Promise<void> {
    const ok = await settings.saveSettings(e.detail);
    if (ok) handleCloseSettings();
  }

  /** 处理黑名单弹窗的保存事件。 */
  async function handleBlacklistSave(e: CustomEvent<AppData["blacklist"]>): Promise<void> {
    const ok = await blacklist.saveBlacklist(e.detail);
    if (ok) handleCloseBlacklist();
  }

  /** 处理模板变更：同步更新 `AppData`（模板列表/启用状态/黑名单）。 */
  function handleTemplatesChange(
    e: CustomEvent<{
      templates: AppData["blacklistTemplates"];
      activeTemplateIds: string[];
      blacklist: AppData["blacklist"];
    }>,
  ): void {
    blacklist.applyTemplatesChange(e.detail);
  }

  /** 处理备注弹窗保存事件。 */
  function handleRemarkSave(e: CustomEvent<string>): void {
    void saveRemark(e.detail);
  }

  /** 转发“开始/暂停”点击到计时器组合式逻辑。 */
  function handleToggleStartPause(): void {
    void timer.toggleStartPause();
  }

  /** 转发“重置”点击到计时器组合式逻辑。 */
  function handleResetTimer(): void {
    void timer.resetTimer();
  }

  /** 转发“跳过”点击到计时器组合式逻辑。 */
  function handleSkipTimer(): void {
    void timer.skipTimer();
  }

  /** 转发“切换置顶”点击到设置组合式逻辑。 */
  function handleToggleAlwaysOnTop(): void {
    void settings.toggleAlwaysOnTop();
  }

  /** 转发“标签选择”到标签组合式逻辑。 */
  function handleTagSelect(tag: string): void {
    void tags.onTagSelectChange(tag);
  }

  /** 转发“标签创建”到标签组合式逻辑。 */
  function handleTagCreate(tag: string): void {
    void tags.addAndSelectTag(tag);
  }
</script>

<main
  class="min-h-screen bg-gradient-to-b from-zinc-50 to-zinc-100 px-4 py-6 text-zinc-900 dark:from-zinc-950 dark:to-zinc-900 dark:text-zinc-50"
>
  <div class="mx-auto w-full max-w-4xl">
    <header class="mb-5 flex items-center justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">番茄钟</h1>
        <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">专注模式下自动终止干扰程序</p>
      </div>
      <div class="flex items-center gap-2 whitespace-nowrap">
        <a
          class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm whitespace-nowrap shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
          href="/history"
        >
          历史记录
        </a>
        <MoreMenu
          alwaysOnTop={$appData?.settings.alwaysOnTop ?? false}
          miniMode={$miniMode}
          onOpenBlacklist={handleOpenBlacklist}
          onOpenSettings={handleOpenSettings}
          onToggleAlwaysOnTop={handleToggleAlwaysOnTop}
          onToggleMiniMode={handleToggleMiniMode}
        />
      </div>
    </header>

    {#if $toastMessage}
      <div class="mb-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
        {$toastMessage}
      </div>
    {/if}

    {#if $appError}
      <div class="rounded-3xl border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-600 dark:text-red-300">
        加载失败：{$appError}
      </div>
    {:else if $appLoading}
      <div
        class="rounded-3xl border border-white/20 bg-white/70 p-4 text-sm text-zinc-600 dark:border-white/10 dark:bg-white/5 dark:text-zinc-300"
      >
        正在加载...
      </div>
    {:else if $appData && $timerSnapshot}
      <section class="grid grid-cols-1 gap-4 md:grid-cols-2">
        <TimerCard
          snapshot={$timerSnapshot}
          requiresAdmin={$killSummary?.requiresAdmin ?? false}
          onToggleStartPause={handleToggleStartPause}
          onReset={handleResetTimer}
          onSkip={handleSkipTimer}
          onRestartAsAdmin={handleRestartAsAdmin}
        />

        <StatsCard
          snapshot={$timerSnapshot}
          tags={$appData.tags}
          onSelectTag={handleTagSelect}
          onCreateTag={handleTagCreate}
        />
      </section>
    {/if}
  </div>
</main>

<SettingsModal
  open={settingsOpen}
  settings={$appData?.settings ?? fallbackSettings}
  on:close={handleCloseSettings}
  on:save={handleSettingsSave}
/>
<BlacklistModal
  open={blacklistOpen}
  blacklist={$appData?.blacklist ?? []}
  locked={$timerSnapshot?.blacklistLocked ?? false}
  templates={$appData?.blacklistTemplates ?? []}
  activeTemplateIds={$appData?.activeTemplateIds ?? []}
  on:close={handleCloseBlacklist}
  on:save={handleBlacklistSave}
  on:templatesChange={handleTemplatesChange}
/>
<RemarkModal open={remarkOpen} event={remarkEvent} on:close={closeRemarkModal} on:save={handleRemarkSave} />
