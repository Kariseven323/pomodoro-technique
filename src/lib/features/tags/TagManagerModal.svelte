<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { deleteTag, renameTag } from "$lib/api/tauri";
  import { applyAppSnapshot } from "$lib/stores/appClient";

  const props = $props<{
    /** 是否打开弹窗。 */
    open: boolean;
    /** 当前标签列表（用于渲染）。 */
    tags: string[];
    /** 提示信息（由上层决定 UI 形态）。 */
    showToast: (message: string) => void;
  }>();

  const dispatch = createEventDispatcher<{ close: void }>();

  let editingTag = $state<string | null>(null);
  let renameDraft = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let wasOpen = $state(false);

  /** 关闭弹窗并清理临时状态。 */
  function close(): void {
    dispatch("close");
  }

  /** 打开某个标签的重命名模式。 */
  function startRename(tag: string): void {
    editingTag = tag;
    renameDraft = tag;
    error = null;
  }

  /** 取消重命名。 */
  function cancelRename(): void {
    editingTag = null;
    renameDraft = "";
    error = null;
  }

  /** 提交重命名：调用后端并同步快照。 */
  async function submitRename(): Promise<void> {
    if (!editingTag) return;
    const from = editingTag.trim();
    const to = renameDraft.trim();
    if (!from || !to) return;
    if (from === to) {
      cancelRename();
      return;
    }

    busy = true;
    error = null;
    try {
      const snapshot = await renameTag(from, to);
      applyAppSnapshot(snapshot);
      props.showToast("已重命名标签");
      cancelRename();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  /** 删除标签：弹确认后调用后端，并同步快照。 */
  async function removeTag(tag: string): Promise<void> {
    const target = tag.trim();
    if (!target) return;
    const ok = window.confirm(`确认删除标签“${target}”吗？（将同步更新历史记录中的该标签）`);
    if (!ok) return;

    busy = true;
    error = null;
    try {
      const snapshot = await deleteTag(target);
      applyAppSnapshot(snapshot);
      props.showToast("已删除标签");
      if (editingTag === target) cancelRename();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  /** 响应 open：打开时清理状态，避免残留。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      editingTag = null;
      renameDraft = "";
      busy = false;
      error = null;
    }
    wasOpen = props.open;
  }

  $effect(onOpenEffect);
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button type="button" class="absolute inset-0 bg-black/30" aria-label="关闭标签管理" onclick={close}></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div class="w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-sm dark:bg-zinc-900">
        <div class="flex items-center justify-between gap-3 border-b border-black/5 p-4 dark:border-white/10">
          <div class="text-base font-semibold text-zinc-900 dark:text-zinc-50">管理标签</div>
          <button
            type="button"
            class="rounded-2xl px-3 py-2 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={close}
          >
            关闭
          </button>
        </div>

        <div class="max-h-[70vh] overflow-y-auto p-3">
          {#if error}
            <div class="mb-3 rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">失败：{error}</div>
          {/if}

          {#if props.tags.length === 0}
            <div class="rounded-2xl bg-black/5 p-3 text-sm text-zinc-600 dark:bg-white/10 dark:text-zinc-300">
              暂无标签
            </div>
          {:else}
            <div class="space-y-2">
              {#each props.tags as t (t)}
                <div class="rounded-2xl border border-black/10 bg-white p-3 dark:border-white/10 dark:bg-zinc-900">
                  {#if editingTag === t}
                    <div class="flex items-center gap-2">
                      <input
                        class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-50"
                        bind:value={renameDraft}
                        disabled={busy}
                      />
                      <button
                        type="button"
                        class="rounded-2xl bg-zinc-900 px-3 py-2 text-sm font-medium text-white shadow-sm hover:bg-zinc-800 disabled:opacity-50 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                        onclick={() => void submitRename()}
                        disabled={busy || !renameDraft.trim()}
                      >
                        保存
                      </button>
                      <button
                        type="button"
                        class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
                        onclick={cancelRename}
                        disabled={busy}
                      >
                        取消
                      </button>
                    </div>
                  {:else}
                    <div class="flex items-center justify-between gap-2">
                      <div class="min-w-0 flex-1 truncate text-sm text-zinc-900 dark:text-zinc-50">{t}</div>
                      <div class="flex items-center gap-2">
                        <button
                          type="button"
                          class="rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 disabled:opacity-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
                          onclick={() => startRename(t)}
                          disabled={busy}
                        >
                          重命名
                        </button>
                        <button
                          type="button"
                          class="rounded-2xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-sm text-red-600 shadow-sm hover:bg-red-500/15 disabled:opacity-50 dark:text-red-300"
                          onclick={() => void removeTag(t)}
                          disabled={busy}
                        >
                          删除
                        </button>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
