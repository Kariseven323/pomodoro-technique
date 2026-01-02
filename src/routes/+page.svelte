<script lang="ts">
  import RemarkModal from "$lib/features/timer/RemarkModal.svelte";
  import InterruptionModal from "$lib/features/timer/InterruptionModal.svelte";
  import CompletionAnimation from "$lib/features/timer/components/CompletionAnimation.svelte";
  import TimerCard from "$lib/features/timer/components/TimerCard.svelte";
  import TagManagerModal from "$lib/features/tags/TagManagerModal.svelte";
  import { appData, appError, appLoading, killSummary, timerSnapshot, workCompleted } from "$lib/stores/appClient";
  import { recordInterruption, setHistoryRemark } from "$lib/api/tauri";
  import type {
    AppData,
    InterruptionDay,
    InterruptionRecord,
    TimerSnapshot,
    WorkCompletedEvent,
  } from "$lib/shared/types";
  import { useTags } from "$lib/composables/useTags";
  import { useTimer } from "$lib/composables/useTimer";
  import { useToast } from "$lib/composables/useToast";

  const { toast: toastMessage, showToast } = useToast();
  const timer = useTimer({ showToast });
  const tags = useTags({ showToast });

  let remarkOpen = $state(false);
  let remarkEvent = $state<WorkCompletedEvent | null>(null);

  let interruptionOpen = $state(false);
  let interruptionAction = $state<"reset" | "skip">("reset");
  let interruptionTag = $state<string>("");
  let interruptionFocusedSeconds = $state<bigint>(0n);
  let interruptionRemainingSeconds = $state<bigint>(0n);

  let tagManagerOpen = $state(false);

  /** 打开标签管理弹窗。 */
  function openTagManager(): void {
    tagManagerOpen = true;
  }

  /** 关闭标签管理弹窗。 */
  function closeTagManager(): void {
    tagManagerOpen = false;
  }

  /** 打开备注弹窗。 */
  function openRemarkModal(e: WorkCompletedEvent): void {
    remarkEvent = e;
    remarkOpen = true;
  }

  /** 关闭备注弹窗。 */
  function closeRemarkModal(): void {
    remarkOpen = false;
    remarkEvent = null;
    workCompleted.set(null);
  }

  /** 保存备注（写入后端并关闭弹窗）。 */
  async function saveRemark(remark: string): Promise<void> {
    if (!remarkEvent) return;
    try {
      await setHistoryRemark(remarkEvent.date, remarkEvent.recordIndex, remark);
      closeRemarkModal();
      showToast("已保存备注");
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }
  }

  /** 监听“工作完成事件”：自动弹出备注窗口。 */
  function onWorkCompletedEffect(): void {
    if ($workCompleted && !remarkOpen) {
      openRemarkModal($workCompleted);
    }
  }

  $effect(onWorkCompletedEffect);

  /** 当后端提示需要提权时，仅展示 toast 提示（入口在黑名单管理页/托盘）。 */
  function onKillSummaryEffect(): void {
    if ($killSummary?.requiresAdmin) {
      showToast("部分进程需要管理员权限才能终止（可在黑名单管理中选择提权重启）");
    }
  }

  $effect(onKillSummaryEffect);

  /** 计算本次工作阶段已专注秒数（仅用于中断弹窗）。 */
  function focusedSeconds(snapshot: TimerSnapshot): bigint {
    const total = BigInt(snapshot.settings.pomodoro) * 60n;
    const remaining =
      typeof snapshot.remainingSeconds === "bigint" ? snapshot.remainingSeconds : BigInt(snapshot.remainingSeconds);
    if (remaining >= total) return 0n;
    return total - remaining;
  }

  /** 判断当前点击是否构成“工作阶段中断”。 */
  function isWorkInterruption(snapshot: TimerSnapshot): boolean {
    return snapshot.phase === "work" && snapshot.blacklistLocked;
  }

  /** 将中断记录追加到全局 `AppData.interruptions`（用于 UI 即时展示）。 */
  function appendInterruptionToStore(record: InterruptionRecord): void {
    appData.update((data): AppData | null => {
      if (!data) return data;
      const date = record.timestamp.slice(0, 10);
      const nextDays: InterruptionDay[] = data.interruptions.map((d) => ({ ...d, records: [...d.records] }));
      const idx = nextDays.findIndex((d) => d.date === date);
      if (idx >= 0) {
        nextDays[idx] = { ...nextDays[idx], records: [...nextDays[idx].records, record] };
      } else {
        nextDays.push({ date, records: [record] });
      }
      return { ...data, interruptions: nextDays, currentCombo: 0 };
    });
  }

  /** 打开中断弹窗并准备展示信息。 */
  function openInterruptionModal(action: "reset" | "skip"): void {
    const snapshot = $timerSnapshot;
    if (!snapshot) return;
    interruptionAction = action;
    interruptionTag = snapshot.currentTag;
    interruptionFocusedSeconds = focusedSeconds(snapshot);
    interruptionRemainingSeconds =
      typeof snapshot.remainingSeconds === "bigint" ? snapshot.remainingSeconds : BigInt(snapshot.remainingSeconds);
    interruptionOpen = true;
  }

  /** 关闭中断弹窗。 */
  function closeInterruptionModal(): void {
    interruptionOpen = false;
  }

  /** 执行“重置/跳过”并在需要时记录中断。 */
  async function performInterruptAction(action: "reset" | "skip", record: boolean, reason: string): Promise<void> {
    const snapshot = $timerSnapshot;
    if (!snapshot) return;
    const shouldRecord = Boolean(record && snapshot.settings.interruption.enabled && isWorkInterruption(snapshot));

    // 即使不记录中断，PRD v4 也要求中断后 Combo 重置。
    if (isWorkInterruption(snapshot)) {
      appData.update((data) => (data ? { ...data, currentCombo: 0 } : data));
    }

    try {
      if (shouldRecord) {
        const r = await recordInterruption(reason, action);
        appendInterruptionToStore(r);
      }
    } catch (e) {
      showToast(e instanceof Error ? e.message : String(e));
    }

    if (action === "reset") {
      await timer.resetTimer();
    } else {
      await timer.skipTimer();
    }
  }

  /** 处理“重置”：若启用中断确认则先弹窗。 */
  function handleResetTimer(): void {
    const snapshot = $timerSnapshot;
    if (!snapshot) return;
    if (
      isWorkInterruption(snapshot) &&
      snapshot.settings.interruption.enabled &&
      snapshot.settings.interruption.confirmOnInterrupt
    ) {
      openInterruptionModal("reset");
      return;
    }
    void performInterruptAction("reset", snapshot.settings.interruption.enabled, "");
  }

  /** 处理“跳过”：若启用中断确认则先弹窗。 */
  function handleSkipTimer(): void {
    const snapshot = $timerSnapshot;
    if (!snapshot) return;
    if (
      isWorkInterruption(snapshot) &&
      snapshot.settings.interruption.enabled &&
      snapshot.settings.interruption.confirmOnInterrupt
    ) {
      openInterruptionModal("skip");
      return;
    }
    void performInterruptAction("skip", snapshot.settings.interruption.enabled, "");
  }

  /** 处理中断弹窗确认：根据用户选择决定是否记录与原因。 */
  function handleInterruptionConfirm(e: CustomEvent<{ record: boolean; reason: string }>): void {
    interruptionOpen = false;
    void performInterruptAction(interruptionAction, e.detail.record, e.detail.reason);
  }

  /** 转发“标签选择”到标签组合式逻辑。 */
  function handleTagSelect(tag: string): void {
    void tags.onTagSelectChange(tag);
  }

  /** 转发“标签创建”到标签组合式逻辑。 */
  function handleTagCreate(tag: string): void {
    void tags.addAndSelectTag(tag);
  }
</script>

<main class="min-h-screen bg-zinc-50 p-4 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50">
  <div class="mx-auto w-full max-w-md">
    {#if $toastMessage}
      <div class="mb-4 rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
        {$toastMessage}
      </div>
    {/if}

    {#if $appError}
      <div class="rounded-2xl bg-red-500/10 p-4 text-sm text-red-600 dark:text-red-300">加载失败：{$appError}</div>
    {:else if $appLoading}
      <div class="rounded-2xl bg-black/5 p-4 text-sm text-zinc-600 dark:bg-white/10 dark:text-zinc-300">
        正在加载...
      </div>
    {:else if $appData && $timerSnapshot}
      <div class="flex flex-col items-center gap-4">
        <TimerCard
          snapshot={$timerSnapshot}
          tags={$appData.tags}
          onToggleStartPause={() => void timer.toggleStartPause()}
          onReset={handleResetTimer}
          onSkip={handleSkipTimer}
          onSelectTag={handleTagSelect}
          onCreateTag={handleTagCreate}
          onManageTags={openTagManager}
        />
      </div>
    {/if}
  </div>
</main>

<TagManagerModal open={tagManagerOpen} tags={$appData?.tags ?? []} {showToast} on:close={closeTagManager} />
<RemarkModal
  open={remarkOpen}
  event={remarkEvent}
  on:close={closeRemarkModal}
  on:save={(e) => void saveRemark(e.detail)}
/>
<InterruptionModal
  open={interruptionOpen}
  action={interruptionAction}
  tag={interruptionTag}
  focusedSeconds={interruptionFocusedSeconds}
  remainingSeconds={interruptionRemainingSeconds}
  on:close={closeInterruptionModal}
  on:confirm={handleInterruptionConfirm}
/>

{#if $timerSnapshot}
  <CompletionAnimation
    enabled={$timerSnapshot.settings.animation.enabled}
    intensity={$timerSnapshot.settings.animation.intensity}
  />
{/if}
