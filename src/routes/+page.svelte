<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type Event as TauriEvent, type UnlistenFn } from "@tauri-apps/api/event";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import BlacklistModal from "$lib/components/BlacklistModal.svelte";
  import {
    getAppSnapshot,
    restartAsAdmin,
    setBlacklist,
    setCurrentTag,
    timerPause,
    timerReset,
    timerSkip,
    timerStart,
    updateSettings,
  } from "$lib/tauriApi";
  import type { AppData, KillSummary, Phase, Settings, TimerSnapshot } from "$lib/types";

  let data = $state<AppData | null>(null);
  let timer = $state<TimerSnapshot | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let settingsOpen = $state(false);
  let blacklistOpen = $state(false);

  let newTag = $state("");
  let toast = $state<string | null>(null);
  let killSummary = $state<KillSummary | null>(null);

  /** 将秒数格式化为 `mm:ss`。 */
  function formatMmSs(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(Math.min(m, 99)).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  /** 将阶段映射为中文展示名。 */
  function phaseLabel(phase: Phase): string {
    if (phase === "work") return "工作";
    if (phase === "shortBreak") return "短休息";
    return "长休息";
  }

  /** 将阶段映射为强调色（Tailwind class）。 */
  function phaseAccentClass(phase: Phase): string {
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

  /** 初始化：从后端加载快照并注册事件监听。 */
  async function initApp(): Promise<void> {
    loading = true;
    error = null;
    try {
      const snapshot = await getAppSnapshot();
      data = snapshot.data;
      timer = snapshot.timer;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  /** 处理后端推送的计时器快照事件。 */
  function onTimerSnapshotEvent(e: TauriEvent<TimerSnapshot>): void {
    timer = e.payload;
  }

  /** 处理后端推送的终止进程结果事件。 */
  function onKillResultEvent(e: TauriEvent<KillSummary>): void {
    killSummary = e.payload;
  }

  /** 注册事件监听并返回清理函数。 */
  function setupListeners(): () => void {
    const unlistenFns: UnlistenFn[] = [];

    void registerListeners(unlistenFns);

    /** 清理已注册的事件监听。 */
    function cleanup(): void {
      for (const fn of unlistenFns) {
        try {
          fn();
        } catch {
          // 忽略卸载时错误
        }
      }
    }

    return cleanup;
  }

  /** 向 Tauri 注册事件监听，并将卸载函数写入数组。 */
  async function registerListeners(unlistenFns: UnlistenFn[]): Promise<void> {
    unlistenFns.push(await listen<TimerSnapshot>("pomodoro://snapshot", onTimerSnapshotEvent));
    unlistenFns.push(await listen<KillSummary>("pomodoro://kill_result", onKillResultEvent));
  }

  /** Svelte 生命周期：挂载时初始化并注册监听。 */
  function onMounted(): () => void {
    void initApp();
    return setupListeners();
  }

  onMount(onMounted);

  /** 切换开始/暂停。 */
  async function toggleStartPause(): Promise<void> {
    if (!timer) return;
    try {
      timer = timer.isRunning ? await timerPause() : await timerStart();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 重置计时器。 */
  async function resetTimer(): Promise<void> {
    try {
      timer = await timerReset();
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 跳过当前阶段。 */
  async function skipTimer(): Promise<void> {
    try {
      timer = await timerSkip();
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
    if (!data) return;
    try {
      const snapshot = await updateSettings(next);
      data = snapshot.data;
      timer = snapshot.timer;
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
    if (!data) return;
    try {
      const saved = await setBlacklist(next);
      data = { ...data, blacklist: saved };
      closeBlacklist();
      showToast("黑名单已更新");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 选择现有标签（通过 `<select>` 的 change 事件）。 */
  async function onTagSelectChange(e: globalThis.Event): Promise<void> {
    if (!data) return;
    const el = e.currentTarget as HTMLSelectElement;
    const tag = el.value;
    if (!tag) return;
    try {
      const snapshot = await setCurrentTag(tag);
      data = snapshot.data;
      timer = snapshot.timer;
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
      data = snapshot.data;
      timer = snapshot.timer;
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

  /** 处理设置弹窗的保存事件。 */
  function onSettingsSave(e: CustomEvent<Settings>): void {
    void saveSettings(e.detail);
  }

  /** 处理黑名单弹窗的保存事件。 */
  function onBlacklistSave(e: CustomEvent<AppData["blacklist"]>): void {
    void saveBlacklist(e.detail);
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
        <button
          class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
          onclick={openBlacklist}
          disabled={!data || !timer}
        >
          管理黑名单
        </button>
        <button
          class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
          onclick={openSettings}
          disabled={!data}
        >
          设置
        </button>
      </div>
    </header>

    {#if error}
      <div class="rounded-3xl border border-red-500/20 bg-red-500/10 p-5 text-sm text-red-700 dark:text-red-200">
        初始化失败：{error}
      </div>
    {:else if loading || !data || !timer}
      <div class="rounded-3xl border border-black/10 bg-white/70 p-5 text-sm text-zinc-600 shadow-sm dark:border-white/10 dark:bg-white/5 dark:text-zinc-300">
        正在加载...
      </div>
    {:else}
      {#if killSummary}
        <div class="mb-4 rounded-3xl border border-amber-500/20 bg-amber-500/10 p-4 text-sm text-amber-800 dark:text-amber-200">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="min-w-0">
              <div class="font-medium">已尝试终止黑名单进程</div>
              <div class="mt-1 text-xs opacity-80">
                {#if killSummary.requiresAdmin}
                  该程序需要管理员权限才能终止。
                {:else}
                  终止操作已完成。
                {/if}
              </div>
            </div>
            {#if killSummary.requiresAdmin}
              <button
                class="rounded-2xl bg-zinc-900 px-4 py-2 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                onclick={onRestartAsAdmin}
              >
                以管理员身份重启
              </button>
            {/if}
          </div>
        </div>
      {/if}

      <section class="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60">
          <div class="flex items-start justify-between">
            <div>
              <div class="text-sm text-zinc-600 dark:text-zinc-300">当前阶段</div>
              <div class={"mt-1 text-2xl font-semibold " + phaseAccentClass(timer.phase)}>{phaseLabel(timer.phase)}</div>
            </div>
            <div class="text-right">
              <div class="text-sm text-zinc-600 dark:text-zinc-300">今日完成</div>
              <div class="mt-1 text-xl font-semibold">{timer.todayStats.total}</div>
            </div>
          </div>

          <div class="mt-5 rounded-3xl bg-black/5 p-5 dark:bg-white/10">
            <div class="text-center text-5xl font-semibold tabular-nums tracking-tight">
              {formatMmSs(timer.remainingSeconds)}
            </div>
            <div class="mt-2 text-center text-xs text-zinc-600 dark:text-zinc-300">
              当前标签：{timer.currentTag || "未标记"}
            </div>
          </div>

          <div class="mt-5 grid grid-cols-4 gap-2">
            <button
              class="col-span-2 rounded-2xl bg-zinc-900 px-4 py-3 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
              onclick={toggleStartPause}
            >
              {timer.isRunning ? "暂停" : "开始"}
            </button>
            <button
              class="rounded-2xl border border-black/10 bg-white/70 px-4 py-3 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
              onclick={resetTimer}
            >
              重置
            </button>
            <button
              class="rounded-2xl border border-black/10 bg-white/70 px-4 py-3 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
              onclick={skipTimer}
            >
              跳过
            </button>
          </div>
        </div>

        <div class="rounded-3xl border border-white/20 bg-white/70 p-5 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60">
          <div class="mb-3 text-sm font-medium text-zinc-900 dark:text-zinc-50">任务标签</div>
          <div class="flex flex-col gap-2">
            <select
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              onchange={onTagSelectChange}
              value={timer.currentTag}
            >
              {#each data.tags as t (t)}
                <option class="bg-white text-zinc-900" value={t}>{t}</option>
              {/each}
            </select>

            <div class="flex gap-2">
              <input
                class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                placeholder="新建标签..."
                bind:value={newTag}
              />
              <button
                class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
                onclick={addAndSelectTag}
              >
                添加
              </button>
            </div>
          </div>

          <div class="mt-6">
            <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">今日统计（按标签）</div>
            {#if timer.todayStats.byTag.length === 0}
              <div class="text-sm text-zinc-500 dark:text-zinc-400">今天还没有完成记录</div>
            {:else}
              <div class="space-y-2">
                {#each timer.todayStats.byTag as item (item.tag)}
                  <div class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
                    <div class="truncate">{item.tag}</div>
                    <div class="tabular-nums">{item.count}</div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="mt-6 text-xs text-zinc-600 dark:text-zinc-300">
            黑名单状态：{timer.blacklistLocked ? "专注期锁定（仅可新增）" : "可编辑"}
          </div>
        </div>
      </section>
    {/if}
  </div>

  {#if data && timer}
    <SettingsModal
      open={settingsOpen}
      settings={data.settings}
      on:close={closeSettings}
      on:save={onSettingsSave}
    />
    <BlacklistModal
      open={blacklistOpen}
      blacklist={data.blacklist}
      locked={timer.blacklistLocked}
      on:close={closeBlacklist}
      on:save={onBlacklistSave}
    />
  {/if}

  {#if toast}
    <div class="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex justify-center px-4">
      <div class="pointer-events-auto rounded-2xl bg-zinc-900 px-4 py-2 text-sm text-white shadow-lg dark:bg-white dark:text-zinc-900">
        {toast}
      </div>
    </div>
  {/if}
</main>
