<script lang="ts">
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import BlacklistModal from "$lib/components/BlacklistModal.svelte";
  import GoalProgress from "$lib/components/GoalProgress.svelte";
  import RemarkModal from "$lib/components/RemarkModal.svelte";
  import { appData, appError, appLoading, killSummary, timerSnapshot, workCompleted } from "$lib/appClient";
  import { miniMode } from "$lib/uiState";
  import { isTauri } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import {
    frontendLog,
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
  let moreMenuOpen = $state(false);
  let moreMenuX = $state(0);
  let moreMenuY = $state(0);
  let moreMenuEl = $state<HTMLDivElement | null>(null);
  let moreMenuAnchorEl = $state<HTMLElement | null>(null);
  let moreMenuMaxHeightPx = $state(0);

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

  /** 判断当前是否运行在 Tauri 宿主环境（非纯浏览器 dev server）。 */
  function isTauriRuntime(): boolean {
    try {
      return isTauri();
    } catch {
      return false;
    }
  }

  /** 基于锚点与菜单尺寸计算“更多”菜单的弹出位置：尽量不超出视口（小窗口/高 DPI 也可用）。 */
  function positionMoreMenu(anchor: HTMLElement, menuWidthPx: number, menuHeightPx: number): void {
    const VIEWPORT_MARGIN_PX = 8;

    const rect = anchor.getBoundingClientRect();
    const spaceBelow = Math.max(0, window.innerHeight - rect.bottom - VIEWPORT_MARGIN_PX);
    const spaceAbove = Math.max(0, rect.top - VIEWPORT_MARGIN_PX);

    const height = Math.min(menuHeightPx, Math.max(0, window.innerHeight - VIEWPORT_MARGIN_PX * 2));
    const preferUp = spaceBelow < height && spaceAbove > spaceBelow;
    const nextY = preferUp ? rect.top - height - VIEWPORT_MARGIN_PX : rect.bottom + VIEWPORT_MARGIN_PX;

    const unclampedX = rect.right - menuWidthPx;
    const maxX = Math.max(VIEWPORT_MARGIN_PX, window.innerWidth - menuWidthPx - VIEWPORT_MARGIN_PX);
    moreMenuX = Math.min(Math.max(unclampedX, VIEWPORT_MARGIN_PX), maxX);

    // 同步对 Y 轴做 clamp，避免菜单在窗口底部被裁剪导致“只显示两项”的错觉。
    const maxY = Math.max(VIEWPORT_MARGIN_PX, window.innerHeight - height - VIEWPORT_MARGIN_PX);
    moreMenuY = Math.min(Math.max(nextY, VIEWPORT_MARGIN_PX), maxY);
  }

  /** 计算并更新“更多”菜单的最大高度与定位（避免 WebView2 对 `vh` 的异常处理导致只显示两项）。 */
  function updateMoreMenuLayout(anchor: HTMLElement, menuEl: HTMLDivElement | null): void {
    const VIEWPORT_MARGIN_PX = 8;
    const maxHeight = Math.max(0, window.innerHeight - VIEWPORT_MARGIN_PX * 2);
    moreMenuMaxHeightPx = Math.floor(maxHeight);

    const width = menuEl?.getBoundingClientRect().width ?? 176;
    const contentHeight = menuEl?.scrollHeight ?? 220;
    const heightForPositioning = Math.min(contentHeight, maxHeight);
    positionMoreMenu(anchor, width, heightForPositioning);
  }

  /** 采集“更多”菜单的布局诊断信息，并写入后端日志（用于 Windows WebView2 环境排查）。 */
  async function logMoreMenuDiagnostics(stage: "pre" | "post"): Promise<void> {
    if (!isTauriRuntime()) return;
    try {
      const anchorRect = moreMenuAnchorEl?.getBoundingClientRect() ?? null;
      const menuRect = moreMenuEl?.getBoundingClientRect() ?? null;
      const cs = moreMenuEl ? window.getComputedStyle(moreMenuEl) : null;
      const buttons = moreMenuEl ? Array.from(moreMenuEl.querySelectorAll("button")) : [];
      const payload = {
        stage,
        time: new Date().toISOString(),
        devicePixelRatio: window.devicePixelRatio,
        inner: { w: window.innerWidth, h: window.innerHeight },
        moreMenuOpen,
        moreMenuX,
        moreMenuY,
        moreMenuMaxHeightPx,
        anchorRect,
        menuRect,
        menu: moreMenuEl
          ? {
              childButtons: moreMenuEl.querySelectorAll("button").length,
              buttonRects: buttons.map((btn: HTMLButtonElement) => {
                const bcs = window.getComputedStyle(btn);
                return {
                  text: btn.textContent?.trim() ?? "",
                  rect: btn.getBoundingClientRect(),
                  display: bcs.display,
                  width: bcs.width,
                  paddingY: `${bcs.paddingTop}/${bcs.paddingBottom}`,
                };
              }),
              scrollHeight: moreMenuEl.scrollHeight,
              clientHeight: moreMenuEl.clientHeight,
              offsetHeight: moreMenuEl.offsetHeight,
              overflowY: cs?.overflowY ?? null,
              maxHeight: cs?.maxHeight ?? null,
            }
          : null,
      };
      await frontendLog("info", `[more_menu_diag] ${JSON.stringify(payload)}`);
    } catch (e) {
      // 日志功能在生产环境可能被禁用；此处降级为控制台输出，避免二次 invoke 导致循环失败。
      console.warn("[more_menu_diag_error]", e);
    }
  }

  /** 打开右上角“更多”菜单。 */
  async function openMoreMenu(e: MouseEvent): Promise<void> {
    const anchor = e.currentTarget;
    if (!(anchor instanceof HTMLElement)) return;
    moreMenuAnchorEl = anchor;
    // 首次打开时使用预估尺寸定位，避免闪烁。
    updateMoreMenuLayout(anchor, null);
    moreMenuOpen = true;
    if (e.shiftKey) {
      await logMoreMenuDiagnostics("pre");
    }
    // 等待菜单渲染后，使用真实尺寸再次定位，确保单列四行不会被窗口裁剪。
    await tick();
    if (!moreMenuOpen || !moreMenuEl || !moreMenuAnchorEl) return;
    updateMoreMenuLayout(moreMenuAnchorEl, moreMenuEl);
    if (e.shiftKey) {
      await logMoreMenuDiagnostics("post");
    }
  }

  /** 关闭右上角“更多”菜单。 */
  function closeMoreMenu(): void {
    moreMenuOpen = false;
  }

  /** 切换右上角“更多”菜单显示状态。 */
  function toggleMoreMenu(e: MouseEvent): void {
    if (moreMenuOpen) {
      closeMoreMenu();
      return;
    }
    void openMoreMenu(e);
  }

  /** 当“更多”菜单打开时，监听窗口尺寸变化并重新定位，避免 DPI/窗口变化导致裁剪。 */
  function onMoreMenuPositionEffect(): (() => void) | void {
    if (!moreMenuOpen || !moreMenuAnchorEl || !moreMenuEl) return;

    /** 处理窗口尺寸变化：使用当前真实尺寸重新定位菜单。 */
    function onResize(): void {
      if (!moreMenuAnchorEl || !moreMenuEl) return;
      updateMoreMenuLayout(moreMenuAnchorEl, moreMenuEl);
    }

    window.addEventListener("resize", onResize);
    return (): void => {
      window.removeEventListener("resize", onResize);
    };
  }

  $effect(onMoreMenuPositionEffect);

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
    if (!isTauriRuntime()) {
      showToast("“管理黑名单”仅桌面端可用，请使用 `bun run tauri dev` 启动。");
      return;
    }
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

  /** 在“更多”菜单中打开黑名单弹窗。 */
  function openBlacklistFromMenu(): void {
    closeMoreMenu();
    openBlacklist();
  }

  /** 在“更多”菜单中打开设置弹窗。 */
  function openSettingsFromMenu(): void {
    closeMoreMenu();
    openSettings();
  }

  /** 在“更多”菜单中切换置顶状态。 */
  function toggleAlwaysOnTopFromMenu(): void {
    closeMoreMenu();
    void toggleAlwaysOnTop();
  }

  /** 在“更多”菜单中切换迷你模式。 */
  function toggleMiniModeFromMenu(): void {
    closeMoreMenu();
    void toggleMiniMode();
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
        <div class="flex items-center gap-2 whitespace-nowrap">
          <a
            class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10 whitespace-nowrap"
            href="/history"
          >
            历史记录
          </a>
          <div class="relative">
            <button
              class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10 whitespace-nowrap"
              aria-haspopup="menu"
              aria-expanded={moreMenuOpen}
              onclick={toggleMoreMenu}
            >
              更多
            </button>
            {#if moreMenuOpen}
              <button
                class="fixed inset-0 z-40"
                type="button"
                aria-label="关闭菜单"
                onclick={closeMoreMenu}
              ></button>
              <div
                bind:this={moreMenuEl}
                class="fixed z-50 flex w-44 flex-col gap-1 overflow-x-hidden overflow-y-auto rounded-2xl border border-black/10 bg-white p-1 shadow-2xl dark:border-white/10 dark:bg-zinc-900"
                style={`left:${moreMenuX}px; top:${moreMenuY}px; max-height:${moreMenuMaxHeightPx}px;`}
              >
                <button
                  class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
                  onclick={openBlacklistFromMenu}
                >
                  管理黑名单
                </button>
                <button
                  class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
                  onclick={openSettingsFromMenu}
                >
                  设置
                </button>
                <button
                  class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
                  onclick={toggleAlwaysOnTopFromMenu}
                >
                  {$appData?.settings.alwaysOnTop ? "取消置顶" : "窗口置顶"}
                </button>
                <button
                  class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
                  onclick={toggleMiniModeFromMenu}
                >
                  {$miniMode ? "退出迷你模式" : "迷你模式"}
                </button>
              </div>
            {/if}
          </div>
        </div>
      </header>

      {#if toast}
        <div class="mb-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
          {toast}
        </div>
      {/if}

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
