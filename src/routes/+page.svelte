<script lang="ts">
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import BlacklistModal from "$lib/components/BlacklistModal.svelte";
  import GoalProgress from "$lib/components/GoalProgress.svelte";
  import RemarkModal from "$lib/components/RemarkModal.svelte";
  import { appData, appError, appLoading, killSummary, timerSnapshot, workCompleted } from "$lib/appClient";
  import { miniMode } from "$lib/uiState";
  import {
    restartAsAdmin,
    setAlwaysOnTop,
    setBlacklist,
    setCurrentTag,
    setHistoryRemark,
    setMiniMode,
    timerPause,
    timerReset,
    timerSkip,
    timerStart,
    updateSettings,
  } from "$lib/tauriApi";
  import type { AppData, Settings, TimerSnapshot, WorkCompletedEvent } from "$lib/types";

  let settingsOpen = $state(false);
  let blacklistOpen = $state(false);
  let remarkOpen = $state(false);
  let remarkEvent = $state<WorkCompletedEvent | null>(null);

  let newTag = $state("");
  let toast = $state<string | null>(null);

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

  /** 将秒数格式化为 `mm:ss`。 */
  function formatMmSs(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(Math.min(m, 99)).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  /** 将阶段映射为中文展示名。 */
  function phaseLabel(phase: TimerSnapshot["phase"]): string {
    if (phase === "work") return "工作";
    if (phase === "shortBreak") return "短休息";
    return "长休息";
  }

  /** 将阶段映射为强调色（Tailwind class）。 */
  function phaseAccentClass(phase: TimerSnapshot["phase"]): string {
    if (phase === "work") return "text-rose-600 dark:text-rose-300";
    if (phase === "shortBreak") return "text-emerald-600 dark:text-emerald-300";
    return "text-sky-600 dark:text-sky-300";
  }

  /** 展示一条短提示，并在一段时间后自动隐藏。 */
  function showToast(message: string): void {
    toast = message;
    window.setTimeout(clearToast, 2200);
  }

  /** 清理 toast。 */
  function clearToast(): void {
    toast = null;
  }

  /** 切换开始/暂停。 */
  async function toggleStartPause(): Promise<void> {
    if (!$timerSnapshot) return;
    try {
      await ($timerSnapshot.isRunning ? timerPause() : timerStart());
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 重置计时器。 */
  async function resetTimer(): Promise<void> {
    try {
      await timerReset();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 跳过当前阶段。 */
  async function skipTimer(): Promise<void> {
    try {
      await timerSkip();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 打开设置弹窗。 */
  function openSettings(): void {
    settingsOpen = true;
  }

  /** 关闭设置弹窗。 */
  function closeSettings(): void {
    settingsOpen = false;
  }

  /** 保存设置并关闭弹窗。 */
  async function saveSettings(next: Settings): Promise<void> {
    if (!$appData) return;
    const prevAlwaysOnTop = $appData.settings.alwaysOnTop;
    try {
      const snapshot = await updateSettings(next);
      appData.set(snapshot.data);
      timerSnapshot.set(snapshot.timer);
      if (next.alwaysOnTop !== prevAlwaysOnTop) {
        await setAlwaysOnTop(next.alwaysOnTop);
      }
      closeSettings();
      showToast("已保存设置");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 打开黑名单弹窗。 */
  function openBlacklist(): void {
    blacklistOpen = true;
  }

  /** 关闭黑名单弹窗。 */
  function closeBlacklist(): void {
    blacklistOpen = false;
  }

  /** 保存黑名单并关闭弹窗。 */
  async function saveBlacklist(next: AppData["blacklist"]): Promise<void> {
    if (!$appData) return;
    try {
      const saved = await setBlacklist(next);
      appData.set({ ...$appData, blacklist: saved });
      closeBlacklist();
      showToast("黑名单已更新");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 选择现有标签（通过 `<select>` 的 change 事件）。 */
  async function onTagSelectChange(e: globalThis.Event): Promise<void> {
    if (!$appData) return;
    const el = e.currentTarget as HTMLSelectElement;
    const tag = el.value;
    if (!tag) return;
    try {
      const snapshot = await setCurrentTag(tag);
      appData.set(snapshot.data);
      timerSnapshot.set(snapshot.timer);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  }

  /** 新建并选择标签。 */
  async function addAndSelectTag(): Promise<void> {
    const tag = newTag.trim();
    if (!tag) return;
    try {
      const snapshot = await setCurrentTag(tag);
      appData.set(snapshot.data);
      timerSnapshot.set(snapshot.timer);
      newTag = "";
      showToast("已添加标签");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 请求以管理员身份重启（用于终止需要提权的进程）。 */
  async function onRestartAsAdmin(): Promise<void> {
    try {
      await restartAsAdmin();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 切换窗口置顶状态。 */
  async function toggleAlwaysOnTop(): Promise<void> {
    if (!$appData) return;
    const next = !$appData.settings.alwaysOnTop;
    try {
      await setAlwaysOnTop(next);
      appData.set({ ...$appData, settings: { ...$appData.settings, alwaysOnTop: next } });
      showToast(next ? "已置顶" : "已取消置顶");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 切换迷你模式。 */
  async function toggleMiniMode(): Promise<void> {
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
  function onSettingsSave(e: CustomEvent<Settings>): void {
    void saveSettings(e.detail);
  }

  /** 处理黑名单弹窗的保存事件。 */
  function onBlacklistSave(e: CustomEvent<AppData["blacklist"]>): void {
    void saveBlacklist(e.detail);
  }

  /** 处理模板变更：同步更新 `AppData`（模板列表/启用状态/黑名单）。 */
  function onTemplatesChange(
    e: CustomEvent<{ templates: AppData["blacklistTemplates"]; activeTemplateIds: string[]; blacklist: AppData["blacklist"] }>,
  ): void {
    if (!$appData) return;
    const activeTemplateId = e.detail.activeTemplateIds[0] ?? null;
    appData.set({
      ...$appData,
      blacklistTemplates: e.detail.templates,
      activeTemplateIds: e.detail.activeTemplateIds,
      activeTemplateId,
      blacklist: e.detail.blacklist,
    });
  }

  /** 处理备注弹窗保存事件。 */
  function onRemarkSave(e: CustomEvent<string>): void {
    void saveRemark(e.detail);
  }
</script>

<main class="min-h-screen bg-gradient-to-b from-zinc-50 to-zinc-100 px-4 py-6 text-zinc-900 dark:from-zinc-950 dark:to-zinc-900 dark:text-zinc-50">
    <div class="mx-auto w-full max-w-4xl">
      <header class="mb-5 flex items-center justify-between gap-3">
        <div>
          <h1 class="text-lg font-semibold tracking-tight">番茄钟</h1>
          <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">专注模式下自动终止干扰程序</p>
        </div>
        <div class="flex items-center gap-2">
          <a
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
            href="/history"
          >
            历史记录
          </a>
          <button
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
            onclick={toggleAlwaysOnTop}
            disabled={!$appData}
            title="窗口置顶"
          >
            {$appData?.settings.alwaysOnTop ? "取消置顶" : "置顶"}
          </button>
          <button
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
            onclick={toggleMiniMode}
            disabled={!$timerSnapshot}
            title="迷你模式"
          >
            迷你模式
          </button>
          <button
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
            onclick={openBlacklist}
            disabled={!$appData || !$timerSnapshot}
          >
            管理黑名单
          </button>
          <button
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
            onclick={openSettings}
            disabled={!$appData}
          >
            设置
          </button>
        </div>
      </header>

      {#if $appError}
        <div class="rounded-3xl border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-600 dark:text-red-300">
          加载失败：{$appError}
        </div>
      {:else if $appLoading}
        <div class="rounded-3xl border border-white/20 bg-white/70 p-4 text-sm text-zinc-600 dark:border-white/10 dark:bg-white/5 dark:text-zinc-300">
          正在加载...
        </div>
      {:else if $appData && $timerSnapshot}
        <section class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60">
            <div class="flex items-start justify-between">
              <div>
                <div class="text-sm text-zinc-600 dark:text-zinc-300">当前阶段</div>
                <div class={"mt-1 text-2xl font-semibold " + phaseAccentClass($timerSnapshot.phase)}>
                  {phaseLabel($timerSnapshot.phase)}
                </div>
              </div>
              <div class="text-right">
                <div class="text-sm text-zinc-600 dark:text-zinc-300">今日完成</div>
                <div class="mt-1 text-xl font-semibold">{$timerSnapshot.todayStats.total}</div>
              </div>
            </div>

            <div class="mt-6 rounded-3xl bg-black/5 p-5 text-center dark:bg-white/10">
              <div class="text-5xl font-bold tabular-nums">{formatMmSs($timerSnapshot.remainingSeconds)}</div>
              <div class="mt-2 text-sm text-zinc-600 dark:text-zinc-300">
                {$timerSnapshot.isRunning ? "计时中..." : "已暂停"}
              </div>
            </div>

            <div class="mt-5 grid grid-cols-3 gap-2">
              <button
                class="rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                onclick={toggleStartPause}
              >
                {$timerSnapshot.isRunning ? "暂停" : "开始"}
              </button>
              <button
                class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-800 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
                onclick={resetTimer}
              >
                重置
              </button>
              <button
                class="rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-800 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
                onclick={skipTimer}
              >
                跳过
              </button>
            </div>

            {#if $killSummary?.requiresAdmin}
              <div class="mt-4 rounded-2xl bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300">
                检测到部分进程需要管理员权限才能终止。
                <button class="ml-2 underline" onclick={onRestartAsAdmin}>以管理员身份重启</button>
              </div>
            {/if}

            {#if toast}
              <div class="mt-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
                {toast}
              </div>
            {/if}
          </div>

          <div class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60">
            <div class="mb-3 text-sm font-medium text-zinc-900 dark:text-zinc-50">目标进度</div>
            <GoalProgress progress={$timerSnapshot.goalProgress} />

            <div class="mt-6">
              <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">任务标签</div>
              <div class="flex flex-col gap-2">
                <select
                  class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                  onchange={onTagSelectChange}
                  value={$timerSnapshot.currentTag}
                >
                  {#each $appData.tags as t (t)}
                    <option class="bg-white text-zinc-900" value={t}>{t}</option>
                  {/each}
                </select>
                <div class="flex items-center gap-2">
                  <input
                    class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                    placeholder="新增标签..."
                    bind:value={newTag}
                  />
                  <button
                    class="shrink-0 rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                    onclick={addAndSelectTag}
                  >
                    添加
                  </button>
                </div>
              </div>
            </div>

            <div class="mt-6">
              <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">今日统计（按标签）</div>
              {#if $timerSnapshot.todayStats.byTag.length === 0}
                <div class="text-sm text-zinc-500 dark:text-zinc-400">今天还没有完成记录</div>
              {:else}
                <div class="space-y-2">
                  {#each $timerSnapshot.todayStats.byTag as item (item.tag)}
                    <div class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
                      <div class="truncate">{item.tag}</div>
                      <div class="tabular-nums">{item.count}</div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="mt-6 text-xs text-zinc-600 dark:text-zinc-300">
              黑名单状态：{$timerSnapshot.blacklistLocked ? "专注期锁定（仅可新增）" : "可编辑"}
            </div>
          </div>
        </section>
      {/if}
    </div>
  </main>

<SettingsModal open={settingsOpen} settings={$appData?.settings ?? fallbackSettings} on:close={closeSettings} on:save={onSettingsSave} />
<BlacklistModal
  open={blacklistOpen}
  blacklist={$appData?.blacklist ?? []}
  locked={$timerSnapshot?.blacklistLocked ?? false}
  templates={$appData?.blacklistTemplates ?? []}
  activeTemplateIds={$appData?.activeTemplateIds ?? []}
  on:close={closeBlacklist}
  on:save={onBlacklistSave}
  on:templatesChange={onTemplatesChange}
/>
<RemarkModal open={remarkOpen} event={remarkEvent} on:close={closeRemarkModal} on:save={onRemarkSave} />
