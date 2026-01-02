<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { audioDelete, audioImport, audioList, frontendLog } from "$lib/api/tauri";
  import type { CustomAudio } from "$lib/shared/types";

  let {
    open: modalOpen,
    enabled = $bindable(),
    currentAudioId = $bindable(),
    autoPlay = $bindable(),
  } = $props<{
    open: boolean;
    enabled: boolean;
    currentAudioId: string;
    autoPlay: boolean;
  }>();

  let audios = $state<CustomAudio[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let importName = $state<string>("");
  let importing = $state(false);

  /** 从文件路径推导默认显示名（优先文件名去扩展名），用于“未填写导入后显示名称”场景。 */
  function defaultDisplayNameFromPath(path: string): string {
    const normalized = path.replaceAll("\\", "/");
    const base = normalized.split("/").pop() ?? "";
    if (!base) return "自定义音效";
    const dot = base.lastIndexOf(".");
    if (dot <= 0) return base;
    return base.slice(0, dot) || "自定义音效";
  }

  /** 拉取音效列表（内置 + 自定义）。 */
  async function loadAudios(): Promise<void> {
    loading = true;
    error = null;
    try {
      audios = await audioList();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      audios = [];
    } finally {
      loading = false;
    }
  }

  /** 选择本地音频文件并导入。 */
  async function pickAndImport(): Promise<void> {
    if (importing) return;
    importing = true;
    error = null;
    notice = null;
    try {
      await frontendLog("info", `[audio_import] click enabled=${enabled}`);
      const selected = await open({
        multiple: false,
        filters: [{ name: "音频文件", extensions: ["mp3", "wav", "ogg", "flac"] }],
      });
      await frontendLog("info", `[audio_import] dialog result=${JSON.stringify(selected)}`);
      if (!selected || Array.isArray(selected)) {
        notice = "已取消选择";
        return;
      }
      const name = importName.trim() || defaultDisplayNameFromPath(selected);
      await frontendLog("info", `[audio_import] importing file=${selected} name=${name}`);
      const created = await audioImport(selected, name);
      await frontendLog("info", `[audio_import] success file=${selected}`);
      notice = "导入成功";
      if (currentAudioId.trim().length === 0) {
        currentAudioId = created.id;
      }
      importName = "";
      await loadAudios();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      await frontendLog("error", `[audio_import] failed: ${error}`);
    } finally {
      importing = false;
    }
  }

  /** 删除一个自定义音效。 */
  async function removeAudio(a: CustomAudio): Promise<void> {
    error = null;
    notice = null;
    try {
      await frontendLog("info", `[audio_delete] click id=${a.id} name=${a.name}`);
      await audioDelete(a.id);
      await frontendLog("info", `[audio_delete] success id=${a.id}`);
      notice = "已删除";
      await loadAudios();
      if (currentAudioId === a.id) {
        currentAudioId = "";
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      await frontendLog("error", `[audio_delete] failed: ${error}`);
    }
  }

  /** 在弹窗打开时刷新音效列表，避免外部导入/删除导致列表过期。 */
  function onOpenEffect(): void {
    if (!modalOpen) return;
    void loadAudios();
  }

  $effect(onOpenEffect);

  onMount(() => {
    void loadAudios();
  });
</script>

<div class="mt-4 space-y-3">
  <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">音效设置</div>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">启用音效</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={enabled} />
  </label>

  <label class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
    <div class="text-zinc-700 dark:text-zinc-200">随番茄自动播放</div>
    <input type="checkbox" class="h-4 w-4" bind:checked={autoPlay} disabled={!enabled} />
  </label>

  <label class="block">
    <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">默认音效</div>
    <select
      class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-zinc-100/90 dark:text-zinc-900"
      bind:value={currentAudioId}
      disabled={!enabled || loading}
    >
      {#if loading}
        <option value={currentAudioId}>加载中...</option>
      {:else if audios.length === 0}
        <option value="">未导入音效</option>
      {:else}
        {#each audios as a (a.id)}
          <option value={a.id}>{a.name}{a.builtin ? "（内置）" : ""}</option>
        {/each}
      {/if}
    </select>
  </label>

  <div class="rounded-2xl bg-black/5 p-3 dark:bg-white/10">
    <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">管理自定义音频</div>

    {#if notice}
      <div class="mb-2 rounded-xl bg-emerald-500/10 px-3 py-2 text-xs text-emerald-700 dark:text-emerald-300">
        {notice}
      </div>
    {/if}
    {#if error}
      <div class="mb-2 rounded-xl bg-red-500/10 px-3 py-2 text-xs text-red-600 dark:text-red-300">
        操作失败：{error}
      </div>
    {/if}

    <div class="flex items-center gap-2">
      <input
        class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none disabled:opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
        placeholder="导入后显示名称（可选）"
        bind:value={importName}
        disabled={importing}
      />
      <button
        class="shrink-0 rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-50 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
        onclick={() => void pickAndImport()}
        disabled={importing}
      >
        {importing ? "导入中..." : "导入"}
      </button>
    </div>

    <div class="mt-3 space-y-2">
      {#if audios.filter((a) => !a.builtin).length === 0}
        <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无自定义音频</div>
      {:else}
        {#each audios.filter((a) => !a.builtin) as a (a.id)}
          <div class="flex items-center justify-between rounded-2xl bg-white/60 px-3 py-2 text-sm dark:bg-white/5">
            <div class="min-w-0 flex-1 truncate text-zinc-800 dark:text-zinc-100">{a.name}</div>
            <button
              class="ml-2 rounded-xl px-2 py-1 text-xs text-red-600 hover:bg-red-500/10 dark:text-red-300 dark:hover:bg-red-500/10"
              onclick={() => void removeAudio(a)}
            >
              删除
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
