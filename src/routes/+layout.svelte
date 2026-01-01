<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";

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
</script>

<slot />
