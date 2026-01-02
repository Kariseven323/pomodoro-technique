<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import { frontendLog, setMiniMode } from "$lib/api/tauri";
  import { appError, appLoading, initAppClient, timerSnapshot } from "$lib/stores/appClient";
  import { installFrontendErrorLogging } from "$lib/utils/frontendDiagnostics";
  import MiniWindow from "$lib/features/timer/MiniWindow.svelte";
  import { miniMode } from "$lib/stores/uiState";
  import type { TimerSnapshot } from "$lib/shared/types";

  const props = $props<{ children?: import("svelte").Snippet }>();

  /** 将 `prefers-color-scheme` 应用到根节点的 `dark` class。 */
  function applyPreferredTheme(): void {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.classList.toggle("dark", prefersDark);
  }

  /** 建立主题监听（系统主题变化时自动更新）。 */
  function setupThemeSync(): () => void {
    const media = window.matchMedia("(prefers-color-scheme: dark)");

    /** 处理系统主题变化事件。 */
    function onThemeChanged(): void {
      applyPreferredTheme();
    }

    applyPreferredTheme();
    media.addEventListener("change", onThemeChanged);

    /** 清理主题监听。 */
    function cleanup(): void {
      media.removeEventListener("change", onThemeChanged);
    }

    return cleanup;
  }

  onMount(setupThemeSync);

  /** Svelte 生命周期：挂载后初始化全局后端快照与事件监听。 */
  function onInitApp(): void {
    void initAppClient();
  }

  /** 初始化 watchdog：避免极端情况下 init 未触发导致永远卡在“正在加载”。 */
  function setupInitWatchdog(): () => void {
    const id = window.setTimeout(() => {
      if ($appLoading) {
        void frontendLog("error", "[frontend] init watchdog fired: still loading after 20s").catch(() => {});
        appError.set("初始化超时：前端未能完成与后端的连接，请查看日志或重启应用。");
        appLoading.set(false);
      }
    }, 20000);
    return (): void => {
      window.clearTimeout(id);
    };
  }

  onMount(() => {
    installFrontendErrorLogging();
    const cleanup = setupInitWatchdog();
    onInitApp();
    return cleanup;
  });

  /** 记录上一次 timerSnapshot，用于判断“自然阶段结束”。 */
  let prevSnapshot: TimerSnapshot | null = null;

  /** 自动退出迷你模式的并发保护标记（不需要响应式）。 */
  let autoExitBusy = false;

  /** 判断一次快照变更是否符合“自然阶段结束”的特征。 */
  function isNaturalPhaseEnd(prev: TimerSnapshot, next: TimerSnapshot): boolean {
    const phaseChanged = prev.phase !== next.phase;
    const prevRemaining =
      typeof prev.remainingSeconds === "bigint" ? prev.remainingSeconds : BigInt(prev.remainingSeconds);
    const prevEnded = prev.isRunning && prevRemaining <= 1n;
    const nextStopped = !next.isRunning;
    return phaseChanged && prevEnded && nextStopped;
  }

  /** 阶段结束时：若未启用自动继续，则自动退出迷你模式（PRD v5）。 */
  async function autoExitMiniModeIfNeeded(prev: TimerSnapshot, next: TimerSnapshot): Promise<void> {
    if (!$miniMode) return;
    if (autoExitBusy) return;
    if (next.settings.autoContinueEnabled) return;
    if (!isNaturalPhaseEnd(prev, next)) return;

    autoExitBusy = true;
    try {
      await setMiniMode(false);
      miniMode.set(false);
    } catch {
      // 忽略自动退出失败：用户仍可通过托盘/双击恢复。
    } finally {
      autoExitBusy = false;
    }
  }

  /** 监听 timerSnapshot：用于迷你模式“阶段结束自动退出”。 */
  function onTimerSnapshotEffect(): void {
    const next = $timerSnapshot;
    const prev = prevSnapshot;
    prevSnapshot = next;
    if (!next || !prev) return;
    void autoExitMiniModeIfNeeded(prev, next);
  }

  $effect(onTimerSnapshotEffect);
</script>

{#if $miniMode}
  <MiniWindow timer={$timerSnapshot} />
{:else}
  <div class="min-h-screen bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50">
    <div class="pb-24">
      {@render props.children?.()}
    </div>
    <TabBar />
  </div>
{/if}
