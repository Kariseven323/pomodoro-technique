<script lang="ts">
  import { onMount } from "svelte";
  import CircularProgress from "$lib/components/CircularProgress.svelte";
  import TagActionSheet from "$lib/features/timer/components/TagActionSheet.svelte";
  import { phaseLabel } from "$lib/utils/phase";
  import { formatMmSs } from "$lib/utils/time";
  import type { TimerSnapshot } from "$lib/shared/types";

  const props = $props<{
    snapshot: TimerSnapshot;
    tags: string[];
    onToggleStartPause: () => void;
    onReset: () => void;
    onSkip: () => void;
    onSelectTag: (tag: string) => void;
    onCreateTag: (tag: string) => void;
    onManageTags: () => void;
  }>();

  let tagSheetOpen = $state(false);
  let ringSize = $state(280);
  let prefersDark = $state(false);

  /** 打开标签选择 action sheet。 */
  function openTagSheet(): void {
    tagSheetOpen = true;
  }

  /** 关闭标签选择 action sheet。 */
  function closeTagSheet(): void {
    tagSheetOpen = false;
  }

  /** 计算当前阶段总秒数（用于进度环）。 */
  function totalSeconds(snapshot: TimerSnapshot): bigint {
    if (snapshot.phase === "work") return BigInt(snapshot.settings.pomodoro) * 60n;
    if (snapshot.phase === "shortBreak") return BigInt(snapshot.settings.shortBreak) * 60n;
    return BigInt(snapshot.settings.longBreak) * 60n;
  }

  /** 将秒数规范化为 bigint（兼容后端序列化为 number 的情况）。 */
  function asBigIntSeconds(v: unknown): bigint {
    if (typeof v === "bigint") return v;
    if (typeof v === "number") return BigInt(Math.max(0, Math.floor(v)));
    if (typeof v === "string" && v.trim()) return BigInt(v);
    return 0n;
  }

  /** 计算 0-1 的进度（剩余秒 -> 已消耗比例）。 */
  function progress01(snapshot: TimerSnapshot): number {
    const total = totalSeconds(snapshot);
    if (total <= 0n) return 0;
    const remainingSeconds = asBigIntSeconds(snapshot.remainingSeconds);
    const remaining = remainingSeconds > total ? total : remainingSeconds;
    const elapsed = total - remaining;
    return Number(elapsed) / Number(total);
  }

  /** 阶段对应的主色（PRD v5）。 */
  function phaseColor(phase: TimerSnapshot["phase"]): string {
    if (phase === "work") return "#EF4444";
    if (phase === "shortBreak") return "#22C55E";
    return "#3B82F6";
  }

  /** 背景环颜色：按明暗模式使用轻微对比度（PRD v5）。 */
  function trackColor(): string {
    return prefersDark ? "rgba(255,255,255,0.1)" : "rgba(0,0,0,0.05)";
  }

  /** 根据窗口宽度计算圆环尺寸（PRD v5：屏幕宽度 60%，最大 280）。 */
  function computeRingSize(): number {
    const next = Math.min(Math.max(0, Math.floor(window.innerWidth * 0.6)), 280);
    return next > 0 ? next : 280;
  }

  /** 初始化：同步圆环尺寸与主题偏好，并建立监听。 */
  function setupUiSync(): () => void {
    ringSize = computeRingSize();
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    prefersDark = media.matches;

    /** 处理窗口尺寸变化：更新圆环尺寸。 */
    function onResize(): void {
      ringSize = computeRingSize();
    }

    /** 处理系统主题变化：更新 trackColor。 */
    function onThemeChanged(): void {
      prefersDark = media.matches;
    }

    window.addEventListener("resize", onResize);
    media.addEventListener("change", onThemeChanged);

    /** 清理监听。 */
    function cleanup(): void {
      window.removeEventListener("resize", onResize);
      media.removeEventListener("change", onThemeChanged);
    }

    return cleanup;
  }

  onMount(setupUiSync);
</script>

<div class="rounded-2xl bg-white p-4 shadow-sm dark:bg-zinc-900">
  <div class="flex flex-col items-center gap-4">
    <div class="relative grid place-items-center" style={`width:min(60vw,280px); height:min(60vw,280px);`}>
      <CircularProgress
        size={ringSize}
        strokeWidth={8}
        progress={progress01(props.snapshot)}
        color={phaseColor(props.snapshot.phase)}
        trackColor={trackColor()}
      />
      <div class="pointer-events-none absolute inset-0 grid place-items-center">
        <div class="text-center">
          <div class="text-4xl font-bold text-zinc-900 tabular-nums dark:text-zinc-50">
            {formatMmSs(props.snapshot.remainingSeconds)}
          </div>
          <div class="mt-1 text-sm text-zinc-500 dark:text-zinc-400">{phaseLabel(props.snapshot.phase)}</div>
        </div>
      </div>
    </div>

    <button
      type="button"
      class="rounded-2xl border border-black/10 bg-white px-4 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
      onclick={openTagSheet}
      aria-haspopup="dialog"
      aria-expanded={tagSheetOpen}
    >
      {props.snapshot.currentTag?.trim() ? props.snapshot.currentTag : "工作"} ▼
    </button>

    <div class="flex items-center justify-center gap-3">
      <button
        type="button"
        class="w-[120px] rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
        onclick={props.onToggleStartPause}
      >
        {props.snapshot.isRunning ? "暂停" : "开始"}
      </button>
      <button
        type="button"
        class="w-16 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
        onclick={props.onReset}
      >
        重置
      </button>
      <button
        type="button"
        class="w-16 rounded-2xl border border-black/10 bg-white px-3 py-2 text-sm text-zinc-800 shadow-sm hover:bg-zinc-50 dark:border-white/10 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-white/5"
        onclick={props.onSkip}
      >
        跳过
      </button>
    </div>
  </div>
</div>

<TagActionSheet
  open={tagSheetOpen}
  tags={props.tags}
  currentTag={props.snapshot.currentTag}
  on:close={closeTagSheet}
  on:select={(e) => {
    closeTagSheet();
    props.onSelectTag(e.detail);
  }}
  on:create={(e) => {
    closeTagSheet();
    props.onCreateTag(e.detail);
  }}
  on:manage={() => {
    closeTagSheet();
    props.onManageTags();
  }}
/>
