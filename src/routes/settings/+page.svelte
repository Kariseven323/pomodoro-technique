<script lang="ts">
  import { onMount } from "svelte";
  import { dev } from "$app/environment";
  import { getVersion } from "@tauri-apps/api/app";
  import SettingsGroup from "$lib/components/SettingsGroup.svelte";
  import SettingsRow from "$lib/components/SettingsRow.svelte";
  import DebugSection from "$lib/features/settings/DebugSection.svelte";
  import AudioLibraryModal from "$lib/features/settings/AudioLibraryModal.svelte";
  import AudioPickerSheet from "$lib/features/settings/AudioPickerSheet.svelte";
  import {
    audioPause,
    audioPlay,
    audioSetVolume,
    getStorePaths,
    openStoreDir,
    setAlwaysOnTop,
    updateSettings,
  } from "$lib/api/tauri";
  import { appData, appError, appLoading, applyAppSnapshot } from "$lib/stores/appClient";
  import type { CustomAudio, Settings, StorePaths } from "$lib/shared/types";
  import { useToast } from "$lib/composables/useToast";

  const { toast: toastMessage, showToast } = useToast();

  let saving = $state(false);
  let savingUiVisible = $state(false);
  let saveError = $state<string | null>(null);
  let pendingSettings = $state<Settings | null>(null);
  let savingUiTimerId = $state<number | null>(null);

  let storePaths = $state<StorePaths | null>(null);
  let storePathsLoading = $state(false);
  let storePathsError = $state<string | null>(null);

  let audioPickerOpen = $state(false);
  let audioLibraryOpen = $state(false);
  let audioPlaying = $state(false);
  let audioError = $state<string | null>(null);
  let appVersion = $state<string>("-");

  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 将数值夹紧到指定整数范围。 */
  function clampInt(v: number, min: number, max: number): number {
    if (!Number.isFinite(v)) return min;
    return Math.max(min, Math.min(max, Math.floor(v)));
  }

  /** 清理“保存中”提示的延迟计时器（避免短暂保存也闪烁 UI）。 */
  function clearSavingUiTimer(): void {
    if (savingUiTimerId === null) return;
    window.clearTimeout(savingUiTimerId);
    savingUiTimerId = null;
  }

  /** 根据 `saving` 状态同步“保存中”提示：仅在保存持续一段时间后才显示。 */
  function onSavingUiEffect(): void {
    clearSavingUiTimer();
    if (!saving) {
      savingUiVisible = false;
      return;
    }
    savingUiTimerId = window.setTimeout(() => {
      if (saving) savingUiVisible = true;
    }, 500);
  }

  $effect(onSavingUiEffect);

  /** 顺序保存设置：合并频繁变更，只提交最后一次。 */
  async function saveSettings(next: Settings): Promise<void> {
    pendingSettings = next;
    if (saving) return;
    saving = true;
    saveError = null;
    try {
      // 关键：必须先“取走当前待保存快照”再 await，否则用户在 await 期间继续修改会被误清空而丢更新。
      while (pendingSettings) {
        const toSave = pendingSettings;
        pendingSettings = null;

        const prevAlwaysOnTop = $appData?.settings.alwaysOnTop ?? false;
        const snapshot = await updateSettings(toSave);
        applyAppSnapshot(snapshot);

        const now = snapshot.data.settings.alwaysOnTop;
        if (prevAlwaysOnTop !== now) {
          await setAlwaysOnTop(now);
        }
      }
    } catch (e) {
      saveError = formatError(e);
    } finally {
      saving = false;
    }
  }

  /** 修改某个顶层数字设置并立即保存。 */
  function updateNumber<K extends keyof Settings>(key: K, value: number, min: number, max: number): void {
    const current = $appData?.settings ?? null;
    if (!current) return;
    const next = { ...current, [key]: clampInt(value, min, max) } as Settings;
    void saveSettings(next);
  }

  /** 修改某个顶层布尔设置并立即保存。 */
  function updateBool<K extends keyof Settings>(key: K, value: boolean): void {
    const current = $appData?.settings ?? null;
    if (!current) return;
    const next = { ...current, [key]: value } as Settings;
    void saveSettings(next);
  }

  /** 修改音效设置并立即保存。 */
  function updateAudioSettings(nextAudio: Settings["audio"]): void {
    const current = $appData?.settings ?? null;
    if (!current) return;
    const next: Settings = { ...current, audio: { ...nextAudio } };
    void saveSettings(next);
  }

  /** 修改动画设置并立即保存。 */
  function updateAnimationSettings(nextAnimation: Settings["animation"]): void {
    const current = $appData?.settings ?? null;
    if (!current) return;
    const next: Settings = { ...current, animation: { ...nextAnimation } };
    void saveSettings(next);
  }

  /** 修改中断设置并立即保存。 */
  function updateInterruptionSettings(nextInterruption: Settings["interruption"]): void {
    const current = $appData?.settings ?? null;
    if (!current) return;
    const next: Settings = { ...current, interruption: { ...nextInterruption } };
    void saveSettings(next);
  }

  /** 加载应用数据根目录路径（用于“关于”展示）。 */
  async function loadStorePaths(): Promise<void> {
    if (storePathsLoading) return;
    storePathsLoading = true;
    storePathsError = null;
    try {
      storePaths = await getStorePaths();
    } catch (e) {
      storePathsError = formatError(e);
      storePaths = null;
    } finally {
      storePathsLoading = false;
    }
  }

  /** 打开数据文件夹（统一入口）。 */
  async function onOpenStoreDir(): Promise<void> {
    try {
      await openStoreDir();
      showToast("已请求打开数据文件夹");
    } catch (e) {
      showToast(formatError(e));
    }
  }

  /** 打开音效选择器。 */
  function openAudioPicker(): void {
    audioPickerOpen = true;
  }

  /** 关闭音效选择器。 */
  function closeAudioPicker(): void {
    audioPickerOpen = false;
  }

  /** 打开自定义音频管理弹窗。 */
  function openAudioLibrary(): void {
    audioLibraryOpen = true;
  }

  /** 关闭自定义音频管理弹窗。 */
  function closeAudioLibrary(): void {
    audioLibraryOpen = false;
  }

  /** 播放/暂停试听（不影响“随番茄自动播放”逻辑）。 */
  async function toggleAudioPreview(): Promise<void> {
    const settings = $appData?.settings ?? null;
    if (!settings) return;
    audioError = null;
    try {
      if (audioPlaying) {
        await audioPause();
        audioPlaying = false;
        return;
      }
      if (!settings.audio.enabled || settings.audio.currentAudioId.trim().length === 0) return;
      const ok = await audioPlay(settings.audio.currentAudioId);
      audioPlaying = ok;
      if (!ok) audioError = "音效未能开始播放（可能是平台不支持或输出设备不可用）";
    } catch (e) {
      audioError = formatError(e);
      audioPlaying = false;
    }
  }

  /** 调整音量：后端立即应用，并写回 settings（用于下次启动恢复）。 */
  async function onVolumeInput(ev: Event): Promise<void> {
    const settings = $appData?.settings ?? null;
    if (!settings) return;
    const v = clampInt(Number((ev.currentTarget as HTMLInputElement).value), 0, 100);
    try {
      await audioSetVolume(v);
      updateAudioSettings({ ...settings.audio, volume: v });
    } catch (e) {
      audioError = formatError(e);
    }
  }

  /** 选择提示音：更新 settings 并在需要时立即切换试听。 */
  async function onSelectAudio(a: CustomAudio | null): Promise<void> {
    const settings = $appData?.settings ?? null;
    if (!settings) return;
    const nextId = a?.id ?? "";
    updateAudioSettings({ ...settings.audio, currentAudioId: nextId });
    closeAudioPicker();
    if (audioPlaying) {
      try {
        if (!nextId) {
          await audioPause();
          audioPlaying = false;
        } else {
          const ok = await audioPlay(nextId);
          audioPlaying = ok;
        }
      } catch (e) {
        audioError = formatError(e);
        audioPlaying = false;
      }
    }
  }

  /** Svelte 生命周期：预加载“关于”信息。 */
  function onMounted(): void {
    void loadStorePaths();
    void loadVersionBestEffort();
  }

  onMount(onMounted);

  /** 尝试读取桌面端版本号（非 Tauri 环境下返回 `-`）。 */
  async function loadVersionBestEffort(): Promise<void> {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = "-";
    }
  }
</script>

<main class="min-h-screen bg-zinc-50 p-4 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50">
  <div class="mx-auto w-full max-w-md">
    <header class="mb-4">
      <h1 class="text-lg font-semibold tracking-tight">设置</h1>
    </header>

    {#if $toastMessage}
      <div class="mb-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
        {$toastMessage}
      </div>
    {/if}
    {#if saveError}
      <div class="mb-4 rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">保存失败：{saveError}</div>
    {/if}
    {#if savingUiVisible}
      <div
        class="fixed top-3 left-1/2 z-50 -translate-x-1/2 rounded-2xl bg-black/5 px-3 py-2 text-xs text-zinc-600 shadow-sm backdrop-blur dark:bg-white/10 dark:text-zinc-300"
        aria-live="polite"
      >
        正在保存...
      </div>
    {/if}

    {#if $appError}
      <div class="rounded-2xl bg-red-500/10 p-4 text-sm text-red-600 dark:text-red-300">加载失败：{$appError}</div>
    {:else if $appLoading || !$appData}
      <div class="rounded-2xl bg-black/5 p-4 text-sm text-zinc-600 dark:bg-white/10 dark:text-zinc-300">
        正在加载...
      </div>
    {:else}
      <SettingsGroup title="计时器">
        <SettingsRow title="工作时长" value={`${$appData.settings.pomodoro} 分钟`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("pomodoro", $appData.settings.pomodoro - 1, 1, 180)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("pomodoro", $appData.settings.pomodoro + 1, 1, 180)}
            >
              +
            </button>
          </div>
        </SettingsRow>

        <SettingsRow title="短休息时长" value={`${$appData.settings.shortBreak} 分钟`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("shortBreak", $appData.settings.shortBreak - 1, 1, 60)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("shortBreak", $appData.settings.shortBreak + 1, 1, 60)}
            >
              +
            </button>
          </div>
        </SettingsRow>

        <SettingsRow title="长休息时长" value={`${$appData.settings.longBreak} 分钟`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("longBreak", $appData.settings.longBreak - 1, 1, 120)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("longBreak", $appData.settings.longBreak + 1, 1, 120)}
            >
              +
            </button>
          </div>
        </SettingsRow>

        <SettingsRow title="长休息间隔" value={`${$appData.settings.longBreakInterval} 个`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("longBreakInterval", $appData.settings.longBreakInterval - 1, 1, 20)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("longBreakInterval", $appData.settings.longBreakInterval + 1, 1, 20)}
            >
              +
            </button>
          </div>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="自动继续">
        <SettingsRow title="启用自动继续">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.autoContinueEnabled}
            onchange={(e) => updateBool("autoContinueEnabled", (e.currentTarget as HTMLInputElement).checked)}
          />
        </SettingsRow>
        <SettingsRow title="连续番茄钟数" value={`${$appData.settings.autoContinuePomodoros} 个`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("autoContinuePomodoros", $appData.settings.autoContinuePomodoros - 1, 1, 20)}
              disabled={!$appData.settings.autoContinueEnabled}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("autoContinuePomodoros", $appData.settings.autoContinuePomodoros + 1, 1, 20)}
              disabled={!$appData.settings.autoContinueEnabled}
            >
              +
            </button>
          </div>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="目标">
        <SettingsRow title="每日目标" value={`${$appData.settings.dailyGoal} 个`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("dailyGoal", $appData.settings.dailyGoal - 1, 0, 1000)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("dailyGoal", $appData.settings.dailyGoal + 1, 0, 1000)}
            >
              +
            </button>
          </div>
        </SettingsRow>
        <SettingsRow title="每周目标" value={`${$appData.settings.weeklyGoal} 个`}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("weeklyGoal", $appData.settings.weeklyGoal - 1, 0, 10000)}
            >
              -
            </button>
            <button
              type="button"
              class="h-8 w-8 rounded-2xl border border-black/10 bg-white text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
              onclick={() => updateNumber("weeklyGoal", $appData.settings.weeklyGoal + 1, 0, 10000)}
            >
              +
            </button>
          </div>
        </SettingsRow>
        <SettingsRow title="窗口置顶">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.alwaysOnTop}
            onchange={(e) => updateBool("alwaysOnTop", (e.currentTarget as HTMLInputElement).checked)}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="音频">
        <SettingsRow title="启用音效">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.audio.enabled}
            onchange={(e) =>
              updateAudioSettings({
                ...$appData.settings.audio,
                enabled: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
        <SettingsRow title="自动播放">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.audio.autoPlay}
            disabled={!$appData.settings.audio.enabled}
            onchange={(e) =>
              updateAudioSettings({
                ...$appData.settings.audio,
                autoPlay: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
        <button type="button" class="w-full text-left" onclick={openAudioPicker}>
          <SettingsRow
            title="提示音选择"
            value={$appData.customAudios.find((a) => a.id === $appData.settings.audio.currentAudioId)?.name ??
              ($appData.settings.audio.currentAudioId.trim().length ? "已选择" : "未选择")}
            chevron
          />
        </button>
        <SettingsRow title="音量" value={`${$appData.settings.audio.volume}%`}>
          <input
            class="w-28 accent-zinc-900 dark:accent-white"
            type="range"
            min="0"
            max="100"
            value={$appData.settings.audio.volume}
            disabled={!$appData.settings.audio.enabled}
            oninput={(e) => void onVolumeInput(e)}
          />
        </SettingsRow>
        <SettingsRow title="试听播放/暂停">
          <button
            type="button"
            class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
            onclick={() => void toggleAudioPreview()}
            disabled={!$appData.settings.audio.enabled || !$appData.settings.audio.currentAudioId.trim()}
          >
            {audioPlaying ? "暂停" : "播放"}
          </button>
        </SettingsRow>
        <button type="button" class="w-full text-left" onclick={openAudioLibrary}>
          <SettingsRow title="管理自定义音频" chevron />
        </button>
        {#if audioError}
          <div class="px-4 pb-3 text-xs text-red-600 dark:text-red-300">{audioError}</div>
        {/if}
      </SettingsGroup>

      <SettingsGroup title="动画">
        <SettingsRow title="启用动画">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.animation.enabled}
            onchange={(e) =>
              updateAnimationSettings({
                ...$appData.settings.animation,
                enabled: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
        <SettingsRow title="连击动画">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.animation.comboEnabled}
            disabled={!$appData.settings.animation.enabled}
            onchange={(e) =>
              updateAnimationSettings({
                ...$appData.settings.animation,
                comboEnabled: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
        <SettingsRow title="动画强度">
          <select
            class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-50"
            value={$appData.settings.animation.intensity}
            disabled={!$appData.settings.animation.enabled}
            onchange={(e) =>
              updateAnimationSettings({
                ...$appData.settings.animation,
                intensity: (e.currentTarget as HTMLSelectElement).value as Settings["animation"]["intensity"],
              })}
          >
            <option value="minimal">简约</option>
            <option value="standard">标准</option>
            <option value="fancy">华丽</option>
          </select>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="中断追踪">
        <SettingsRow title="启用中断追踪">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.interruption.enabled}
            onchange={(e) =>
              updateInterruptionSettings({
                ...$appData.settings.interruption,
                enabled: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
        <SettingsRow title="中断时确认">
          <input
            type="checkbox"
            class="h-5 w-5"
            checked={$appData.settings.interruption.confirmOnInterrupt}
            disabled={!$appData.settings.interruption.enabled}
            onchange={(e) =>
              updateInterruptionSettings({
                ...$appData.settings.interruption,
                confirmOnInterrupt: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup>
        <a href="/settings/blacklist" class="block">
          <SettingsRow title="黑名单管理" chevron />
        </a>
      </SettingsGroup>

      <SettingsGroup title="关于">
        <SettingsRow
          title="数据存储路径（总入口，只读）"
          value={storePaths?.storeDirPath ?? (storePathsLoading ? "加载中..." : "")}
        />
        {#if storePathsError}
          <div class="px-4 pb-3 text-xs text-red-600 dark:text-red-300">失败：{storePathsError}</div>
        {/if}
        <SettingsRow title="打开数据文件夹">
          <button
            type="button"
            class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
            onclick={() => void onOpenStoreDir()}
          >
            打开
          </button>
        </SettingsRow>
        <SettingsRow title="版本" value={appVersion} />
      </SettingsGroup>

      {#if dev}
        <div class="mt-6">
          <DebugSection />
        </div>
      {/if}
    {/if}
  </div>
</main>

<AudioPickerSheet
  open={audioPickerOpen}
  currentAudioId={$appData?.settings.audio.currentAudioId ?? ""}
  audios={$appData?.customAudios ?? []}
  on:close={closeAudioPicker}
  on:select={(e) => void onSelectAudio(e.detail)}
/>
<AudioLibraryModal open={audioLibraryOpen} on:close={closeAudioLibrary} {showToast} />
