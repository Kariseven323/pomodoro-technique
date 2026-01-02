<script lang="ts">
  import { exitApp, timerPause, timerStart, setMiniMode } from "$lib/api/tauri";
  import { miniMode } from "$lib/stores/uiState";
  import type { TimerSnapshot } from "$lib/shared/types";
  import { phaseLabel } from "$lib/utils/phase";
  import { formatMmSs } from "$lib/utils/time";

  const props = $props<{ timer: TimerSnapshot | null }>();

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);

  /** 关闭右键菜单。 */
  function closeMenu(): void {
    menuOpen = false;
  }

  /** 打开右键菜单。 */
  function onContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  /** 恢复主窗口：退出迷你模式并关闭菜单。 */
  async function restore(): Promise<void> {
    closeMenu();
    await setMiniMode(false);
    miniMode.set(false);
  }

  /** 暂停/继续：根据当前运行状态切换。 */
  async function togglePause(): Promise<void> {
    closeMenu();
    if (!props.timer) return;
    if (props.timer.isRunning) {
      await timerPause();
    } else {
      await timerStart();
    }
  }

  /** 退出应用。 */
  async function quitApp(): Promise<void> {
    closeMenu();
    await exitApp();
  }

  /** 双击恢复主窗口。 */
  function onDblClick(): void {
    void restore();
  }
</script>

<main
  class="h-screen w-screen bg-zinc-950 text-zinc-50 select-none"
  data-tauri-drag-region
  ondblclick={onDblClick}
  oncontextmenu={onContextMenu}
>
  <div class="flex h-full items-center justify-between gap-3 px-3" data-tauri-drag-region>
    <div class="min-w-0">
      <div class="text-xs text-zinc-300" data-tauri-drag-region>
        {#if props.timer}
          {phaseLabel(props.timer.phase)}
        {:else}
          —
        {/if}
      </div>
      <div class="truncate text-[10px] text-zinc-400" data-tauri-drag-region>双击恢复 · 右键菜单</div>
    </div>
    <div class="text-3xl font-semibold tabular-nums" data-tauri-drag-region>
      {#if props.timer}
        {formatMmSs(props.timer.remainingSeconds)}
      {:else}
        00:00
      {/if}
    </div>
  </div>

  {#if menuOpen}
    <button class="fixed inset-0 z-40" type="button" aria-label="关闭菜单" onclick={closeMenu}></button>
    <div
      class="fixed z-50 w-36 overflow-hidden rounded-xl border border-white/10 bg-zinc-900 shadow-2xl"
      style={`left:${menuX}px; top:${menuY}px;`}
    >
      <button class="w-full px-3 py-2 text-left text-sm hover:bg-white/10" onclick={restore}>恢复</button>
      <button class="w-full px-3 py-2 text-left text-sm hover:bg-white/10" onclick={togglePause}>
        {#if props.timer?.isRunning}暂停{:else}开始{/if}
      </button>
      <button class="w-full px-3 py-2 text-left text-sm text-red-300 hover:bg-white/10" onclick={quitApp}>退出</button>
    </div>
  {/if}
</main>
