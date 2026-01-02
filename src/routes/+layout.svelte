<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { initAppClient, timerSnapshot } from "$lib/stores/appClient";
  import MiniWindow from "$lib/features/timer/MiniWindow.svelte";
  import { miniMode } from "$lib/stores/uiState";

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

  onMount(onInitApp);
</script>

{#if $miniMode}
  <MiniWindow timer={$timerSnapshot} />
{:else}
  <slot />
{/if}
