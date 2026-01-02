<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import confetti from "canvas-confetti";
  import { milestoneReached, pomodoroCompleted } from "$lib/stores/appClient";
  import type { AnimationIntensity, MilestoneReachedPayload, PomodoroCompletedPayload } from "$lib/shared/types";

  const props = $props<{ enabled: boolean; intensity: AnimationIntensity }>();

  let lastTotal = $state<bigint | null>(null);
  let active = $state(false);
  let cancelFrame = $state<number | null>(null);

  /** 将动画强度映射到粒子数量（越高越华丽）。 */
  function particleCount(intensity: AnimationIntensity): number {
    if (intensity === "minimal") return 60;
    if (intensity === "fancy") return 220;
    return 120;
  }

  /** 将动画强度映射到扩散角度（越高越夸张）。 */
  function particleSpread(intensity: AnimationIntensity): number {
    if (intensity === "minimal") return 60;
    if (intensity === "fancy") return 110;
    return 85;
  }

  /** 触发“粒子爆发”动画（单次番茄完成）。 */
  function burst(intensity: AnimationIntensity): void {
    confetti({
      particleCount: particleCount(intensity),
      spread: particleSpread(intensity),
      startVelocity: intensity === "minimal" ? 38 : intensity === "fancy" ? 55 : 46,
      origin: { x: 0.5, y: 0.38 },
      gravity: 0.9,
      ticks: 180,
    });
  }

  /** 触发“烟花”动画（每日目标/里程碑达成）。 */
  function fireworks(intensity: AnimationIntensity, seconds: number): void {
    const end = Date.now() + seconds * 1000;
    const baseCount = Math.round(particleCount(intensity) / 4);

    /** 逐帧发射烟花，直到到达结束时间。 */
    function frame(): void {
      confetti({
        particleCount: baseCount,
        spread: particleSpread(intensity),
        startVelocity: intensity === "minimal" ? 42 : intensity === "fancy" ? 60 : 50,
        origin: { x: Math.random() * 0.8 + 0.1, y: Math.random() * 0.35 + 0.05 },
        gravity: 1,
        ticks: 180,
      });

      if (Date.now() < end) {
        cancelFrame = requestAnimationFrame(frame);
      } else {
        cancelFrame = null;
        active = false;
      }
    }

    active = true;
    frame();
  }

  /** 结束动画：停止循环并清理粒子。 */
  function stop(): void {
    active = false;
    if (cancelFrame) {
      cancelAnimationFrame(cancelFrame);
      cancelFrame = null;
    }
    confetti.reset();
  }

  /** 监听全局点击：在动画进行时允许点击跳过（不阻塞用户操作）。 */
  function onPointerDown(): void {
    if (!active) return;
    stop();
  }

  /** 响应“番茄完成”事件：触发粒子爆发/每日目标烟花。 */
  function onPomodoroCompletedEffect(): void {
    if (!props.enabled) return;
    const payload: PomodoroCompletedPayload | null = $pomodoroCompleted;
    if (!payload) return;
    if (lastTotal === payload.total) return;
    lastTotal = payload.total;

    burst(props.intensity);
    if (payload.dailyGoalReached) {
      fireworks(props.intensity, 2.6);
    }
  }

  /** 响应“里程碑达成”事件：触发更强的烟花。 */
  function onMilestoneReachedEffect(): void {
    if (!props.enabled) return;
    const payload: MilestoneReachedPayload | null = $milestoneReached;
    if (!payload) return;
    fireworks(props.intensity, 3.0);
  }

  $effect(onPomodoroCompletedEffect);
  $effect(onMilestoneReachedEffect);

  /** 生命周期：挂载时注册全局点击监听，卸载时清理。 */
  function onLifecycle(): void {
    window.addEventListener("pointerdown", onPointerDown, { capture: true });
    onDestroy(() => {
      window.removeEventListener("pointerdown", onPointerDown, { capture: true });
      stop();
    });
  }

  onMount(onLifecycle);
</script>

<!--
  本组件不渲染任何可交互 UI：动画通过 canvas-confetti 全局绘制，且不会阻塞用户操作。
  “点击跳过”通过全局 pointerdown 监听实现。
-->
