<script lang="ts">
  const props = $props<{ combo: number; enabled: boolean }>();

  let bouncing = $state(false);
  let lastCombo = $state<number>(0);

  /** 在 combo 变化时触发一次弹跳动画。 */
  function triggerBounce(): void {
    bouncing = true;
    window.setTimeout(() => {
      bouncing = false;
    }, 260);
  }

  /** 监听 combo 变化：仅在 2+ 时触发动画。 */
  function onComboEffect(): void {
    if (!props.enabled) return;
    if (props.combo < 2) {
      lastCombo = props.combo;
      bouncing = false;
      return;
    }
    if (props.combo !== lastCombo) {
      lastCombo = props.combo;
      triggerBounce();
    }
  }

  $effect(onComboEffect);
</script>

{#if props.enabled && props.combo >= 2}
  <div
    class={"rounded-full bg-amber-500/15 px-3 py-1 text-xs font-semibold text-amber-700 shadow-sm ring-1 ring-amber-500/25 dark:text-amber-200 " +
      (bouncing ? "animate-[combo-bounce_260ms_ease-out]" : "")}
    title="连续完成连击"
  >
    x{props.combo}
  </div>
{/if}

<style>
  @keyframes combo-bounce {
    0% {
      transform: scale(0.9);
    }
    55% {
      transform: scale(1.18);
    }
    100% {
      transform: scale(1);
    }
  }
</style>
