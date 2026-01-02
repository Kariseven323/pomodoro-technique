<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CustomAudio } from "$lib/shared/types";

  const props = $props<{
    /** 是否打开。 */
    open: boolean;
    /** 当前选中的音效 id。 */
    currentAudioId: string;
    /** 音效列表（内置+自定义）。 */
    audios: CustomAudio[];
  }>();

  const dispatch = createEventDispatcher<{
    close: void;
    select: CustomAudio | null;
  }>();

  /** 关闭选择器（不变更选择）。 */
  function close(): void {
    dispatch("close");
  }

  /** 选择某个音效并关闭。 */
  function choose(a: CustomAudio): void {
    dispatch("select", a);
  }

  /** 清空选择并关闭。 */
  function clear(): void {
    dispatch("select", null);
  }
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button type="button" class="absolute inset-0 bg-black/30" aria-label="关闭音效选择" onclick={close}></button>
    <div class="absolute inset-x-0 bottom-0 p-4">
      <div class="mx-auto w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-sm dark:bg-zinc-900">
        <div class="border-b border-black/5 p-4 dark:border-white/10">
          <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">提示音选择</div>
          <div class="mt-1 text-xs text-zinc-500 dark:text-zinc-400">选择一个音效作为提示音</div>
        </div>

        <div class="max-h-[55vh] overflow-y-auto p-2">
          <button
            type="button"
            class={"flex w-full items-center justify-between rounded-2xl px-4 py-3 text-left text-sm " +
              (!props.currentAudioId
                ? "bg-black/5 text-zinc-900 dark:bg-white/10 dark:text-zinc-50"
                : "text-zinc-800 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10")}
            onclick={clear}
          >
            <span class="truncate">不使用提示音</span>
            {#if !props.currentAudioId}
              <span class="text-xs text-zinc-500 dark:text-zinc-400">当前</span>
            {/if}
          </button>

          {#each props.audios as a (a.id)}
            <button
              type="button"
              class={"flex w-full items-center justify-between rounded-2xl px-4 py-3 text-left text-sm " +
                (a.id === props.currentAudioId
                  ? "bg-black/5 text-zinc-900 dark:bg-white/10 dark:text-zinc-50"
                  : "text-zinc-800 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10")}
              onclick={() => choose(a)}
            >
              <span class="truncate">{a.name}{a.builtin ? "（内置）" : ""}</span>
              {#if a.id === props.currentAudioId}
                <span class="text-xs text-zinc-500 dark:text-zinc-400">当前</span>
              {/if}
            </button>
          {/each}
        </div>

        <div class="border-t border-black/5 p-2 dark:border-white/10">
          <button
            type="button"
            class="w-full rounded-2xl px-4 py-3 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={close}
          >
            取消
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
