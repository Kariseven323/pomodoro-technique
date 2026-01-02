<script lang="ts">
  import { onMount } from "svelte";
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { audioDelete, audioImport, audioList, frontendLog } from "$lib/api/tauri";
  import type { CustomAudio } from "$lib/shared/types";

  const props = $props<{
    /** 是否打开弹窗。 */
    open: boolean;
    /** 提示信息（由上层决定 UI 形态）。 */
    showToast: (message: string) => void;
  }>();

  const dispatch = createEventDispatcher<{ close: void }>();

  let audios = $state<CustomAudio[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let importName = $state<string>("");
  let importing = $state(false);
  let wasOpen = $state(false);

  /** 关闭弹窗。 */
  function close(): void {
    dispatch("close");
  }

  /** 将未知异常转为可读字符串。 */
  function formatError(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** 从文件路径推导默认显示名（优先文件名去扩展名）。 */
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
      error = formatError(e);
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
      await frontendLog("info", "[audio_import] click");
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
      const created = await audioImport(selected, name);
      notice = `已导入：${created.name}`;
      importName = "";
      await loadAudios();
      props.showToast("导入成功");
    } catch (e) {
      error = formatError(e);
      props.showToast(error);
    } finally {
      importing = false;
    }
  }

  /** 删除一个自定义音效。 */
  async function removeAudio(a: CustomAudio): Promise<void> {
    if (a.builtin) return;
    const ok = window.confirm(`确认删除音效“${a.name}”吗？`);
    if (!ok) return;
    error = null;
    notice = null;
    try {
      await audioDelete(a.id);
      notice = "已删除";
      await loadAudios();
      props.showToast("已删除音效");
    } catch (e) {
      error = formatError(e);
      props.showToast(error);
    }
  }

  /** 响应 open：打开时刷新列表，关闭时清理提示状态。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      void loadAudios();
      notice = null;
      error = null;
    }
    if (!props.open && wasOpen) {
      notice = null;
      error = null;
    }
    wasOpen = props.open;
  }

  $effect(onOpenEffect);

  onMount(() => {
    void loadAudios();
  });
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button type="button" class="absolute inset-0 bg-black/30" aria-label="关闭音频管理" onclick={close}></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div class="w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-sm dark:bg-zinc-900">
        <div class="flex items-center justify-between gap-3 border-b border-black/5 p-4 dark:border-white/10">
          <div class="text-base font-semibold text-zinc-900 dark:text-zinc-50">管理自定义音频</div>
          <button
            type="button"
            class="rounded-2xl px-3 py-2 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={close}
          >
            关闭
          </button>
        </div>

        <div class="p-4">
          {#if notice}
            <div class="mb-3 rounded-2xl bg-emerald-500/10 p-3 text-xs text-emerald-700 dark:text-emerald-300">
              {notice}
            </div>
          {/if}
          {#if error}
            <div class="mb-3 rounded-2xl bg-red-500/10 p-3 text-xs text-red-600 dark:text-red-300">
              失败：{error}
            </div>
          {/if}

          <div class="flex items-center gap-2">
            <input
              class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-50"
              placeholder="导入后显示名称（可选）"
              bind:value={importName}
              disabled={importing}
            />
            <button
              type="button"
              class="shrink-0 rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow-sm hover:bg-zinc-800 disabled:opacity-50 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
              onclick={() => void pickAndImport()}
              disabled={importing}
            >
              {importing ? "导入中..." : "导入"}
            </button>
          </div>

          <div class="mt-4 space-y-2">
            {#if loading}
              <div class="text-sm text-zinc-500 dark:text-zinc-400">加载中...</div>
            {:else if audios.filter((a) => !a.builtin).length === 0}
              <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无自定义音频</div>
            {:else}
              {#each audios.filter((a) => !a.builtin) as a (a.id)}
                <div
                  class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10"
                >
                  <div class="min-w-0 flex-1 truncate text-zinc-800 dark:text-zinc-100">{a.name}</div>
                  <button
                    type="button"
                    class="ml-2 rounded-2xl px-2 py-1 text-xs text-red-600 hover:bg-red-500/10 dark:text-red-300 dark:hover:bg-red-500/10"
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
    </div>
  </div>
{/if}
