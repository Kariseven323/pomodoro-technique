<script lang="ts">
  import { get } from "svelte/store";
  import { audioPause, audioPlay, audioSetVolume, updateSettings } from "$lib/api/tauri";
  import { appData, applyAppSnapshot, timerSnapshot } from "$lib/stores/appClient";
  import type { CustomAudio, Settings } from "$lib/shared/types";

  const props = $props<{ settings: Settings }>();

  let audios = $state<CustomAudio[]>([]);
  let error = $state<string | null>(null);
  let playing = $state(false);

  /** 从全局 `appData.customAudios` 同步音效列表，确保“导入后返回主界面”能立即刷新。 */
  function syncAudiosEffect(): void {
    audios = $appData?.customAudios ?? [];
  }

  $effect(syncAudiosEffect);

  /** 将 settings 的 audio 字段变更写回后端并同步到全局 store。 */
  async function saveAudioSettings(next: Settings["audio"]): Promise<void> {
    const current = get(appData);
    if (!current) return;
    const settings: Settings = { ...current.settings, audio: { ...next } };
    const snapshot = await updateSettings(settings);
    applyAppSnapshot(snapshot);
  }

  /** 切换“启用音效”。 */
  async function toggleEnabled(): Promise<void> {
    const next = { ...props.settings.audio, enabled: !props.settings.audio.enabled };
    await saveAudioSettings(next);
    if (!next.enabled) {
      try {
        await audioPause();
        playing = false;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
    }
  }

  /** 切换“随番茄自动播放”。 */
  async function toggleAutoPlay(): Promise<void> {
    const next = { ...props.settings.audio, autoPlay: !props.settings.audio.autoPlay };
    await saveAudioSettings(next);
    if (
      next.autoPlay &&
      next.currentAudioId.trim().length > 0 &&
      $timerSnapshot?.phase === "work" &&
      $timerSnapshot?.isRunning
    ) {
      try {
        const ok = await audioPlay(next.currentAudioId);
        playing = ok;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        playing = false;
      }
    }
  }

  /** 切换音效下拉菜单（若正在播放则立即切换）。 */
  async function onSelectAudio(ev: Event): Promise<void> {
    const id = (ev.currentTarget as HTMLSelectElement).value;
    const next = { ...props.settings.audio, currentAudioId: id };
    await saveAudioSettings(next);
    if (playing) {
      try {
        if (id.trim().length === 0) {
          await audioPause();
          playing = false;
        } else {
          const ok = await audioPlay(id);
          playing = ok;
        }
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        playing = false;
      }
    }
  }

  /** 播放/暂停切换。 */
  async function togglePlayPause(): Promise<void> {
    if (playing) {
      try {
        await audioPause();
        playing = false;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
      return;
    }
    if (props.settings.audio.currentAudioId.trim().length === 0) return;
    try {
      const ok = await audioPlay(props.settings.audio.currentAudioId);
      playing = ok;
      if (!ok) {
        error = "音效未能开始播放（可能是平台不支持或输出设备不可用）";
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      playing = false;
    }
  }

  /** 调整音量：后端立即应用，并同步写回 settings（用于下次启动恢复）。 */
  async function onVolumeInput(ev: Event): Promise<void> {
    const v = Number((ev.currentTarget as HTMLInputElement).value);
    await audioSetVolume(v);
    const current = get(appData);
    if (!current) return;
    appData.set({ ...current, settings: { ...current.settings, audio: { ...current.settings.audio, volume: v } } });
  }
</script>

<div class="mt-4 rounded-2xl bg-black/5 p-3 dark:bg-white/10">
  <div class="mb-2 flex items-center justify-between gap-2">
    <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">音效</div>
    <div class="flex items-center gap-2">
      <button
        class="rounded-xl border border-black/10 bg-white/70 px-3 py-1 text-xs text-zinc-700 hover:bg-white disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
        onclick={() => void togglePlayPause()}
        disabled={!props.settings.audio.enabled || props.settings.audio.currentAudioId.trim().length === 0}
      >
        {playing ? "暂停" : "播放"}
      </button>
      <label class="flex items-center gap-2 text-xs text-zinc-600 dark:text-zinc-300">
        <span>启用</span>
        <input
          type="checkbox"
          class="h-4 w-4"
          checked={props.settings.audio.enabled}
          onchange={() => void toggleEnabled()}
        />
      </label>
    </div>
  </div>

  {#if error}
    <div class="mb-2 rounded-xl bg-red-500/10 px-3 py-2 text-xs text-red-600 dark:text-red-300">加载失败：{error}</div>
  {/if}

  <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
    <label class="block">
      <div class="mb-1 text-xs text-zinc-600 dark:text-zinc-300">音效选择</div>
      <select
        class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-zinc-100/90 dark:text-zinc-900"
        value={props.settings.audio.currentAudioId}
        disabled={!props.settings.audio.enabled}
        onchange={(ev) => void onSelectAudio(ev)}
      >
        {#if audios.length === 0}
          <option value="">未导入音效</option>
        {:else}
          {#each audios as a (a.id)}
            <option value={a.id}>{a.name}{a.builtin ? "（内置）" : ""}</option>
          {/each}
        {/if}
      </select>
    </label>

    <label class="block">
      <div class="mb-1 flex items-center justify-between text-xs text-zinc-600 dark:text-zinc-300">
        <span>音量</span>
        <span class="tabular-nums">{props.settings.audio.volume}%</span>
      </div>
      <input
        class="w-full accent-zinc-900 dark:accent-white"
        type="range"
        min="0"
        max="100"
        value={props.settings.audio.volume}
        disabled={!props.settings.audio.enabled}
        oninput={(ev) => void onVolumeInput(ev)}
      />
    </label>
  </div>

  <div class="mt-3 flex items-center justify-between rounded-2xl bg-white/60 px-3 py-2 text-xs dark:bg-white/5">
    <div class="text-zinc-600 dark:text-zinc-300">随番茄自动播放（专注开始播放，休息/暂停自动停止）</div>
    <input
      type="checkbox"
      class="h-4 w-4"
      checked={props.settings.audio.autoPlay}
      disabled={!props.settings.audio.enabled}
      onchange={() => void toggleAutoPlay()}
    />
  </div>
</div>
