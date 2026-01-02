<script lang="ts">
  import { tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { frontendLog } from "$lib/api/tauri";

  const props = $props<{
    /** 当前是否置顶（用于菜单文案）。 */
    alwaysOnTop: boolean;
    /** 当前是否为迷你模式（用于菜单文案）。 */
    miniMode: boolean;
    /** 打开黑名单弹窗（由上层决定是否在 Tauri 环境下可用）。 */
    onOpenBlacklist: () => void;
    /** 打开设置弹窗。 */
    onOpenSettings: () => void;
    /** 切换置顶。 */
    onToggleAlwaysOnTop: () => void;
    /** 切换迷你模式。 */
    onToggleMiniMode: () => void;
  }>();

  let open = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuEl = $state<HTMLDivElement | null>(null);
  let anchorEl = $state<HTMLElement | null>(null);
  let maxHeightPx = $state(0);

  /** 判断当前是否运行在 Tauri 宿主环境（非纯浏览器 dev server）。 */
  function isTauriRuntime(): boolean {
    try {
      return isTauri();
    } catch {
      return false;
    }
  }

  /** 基于锚点与菜单尺寸计算弹出位置：尽量不超出视口（小窗口/高 DPI 也可用）。 */
  function positionMenu(anchor: HTMLElement, menuWidthPx: number, menuHeightPx: number): void {
    const VIEWPORT_MARGIN_PX = 8;

    const rect = anchor.getBoundingClientRect();
    const spaceBelow = Math.max(0, window.innerHeight - rect.bottom - VIEWPORT_MARGIN_PX);
    const spaceAbove = Math.max(0, rect.top - VIEWPORT_MARGIN_PX);

    const height = Math.min(menuHeightPx, Math.max(0, window.innerHeight - VIEWPORT_MARGIN_PX * 2));
    const preferUp = spaceBelow < height && spaceAbove > spaceBelow;
    const nextY = preferUp ? rect.top - height - VIEWPORT_MARGIN_PX : rect.bottom + VIEWPORT_MARGIN_PX;

    const unclampedX = rect.right - menuWidthPx;
    const maxX = Math.max(VIEWPORT_MARGIN_PX, window.innerWidth - menuWidthPx - VIEWPORT_MARGIN_PX);
    menuX = Math.min(Math.max(unclampedX, VIEWPORT_MARGIN_PX), maxX);

    const maxY = Math.max(VIEWPORT_MARGIN_PX, window.innerHeight - height - VIEWPORT_MARGIN_PX);
    menuY = Math.min(Math.max(nextY, VIEWPORT_MARGIN_PX), maxY);
  }

  /** 计算并更新菜单最大高度与定位（避免 WebView2 对 `vh` 的异常处理导致裁剪）。 */
  function updateMenuLayout(anchor: HTMLElement, el: HTMLDivElement | null): void {
    const VIEWPORT_MARGIN_PX = 8;
    const maxHeight = Math.max(0, window.innerHeight - VIEWPORT_MARGIN_PX * 2);
    maxHeightPx = Math.floor(maxHeight);

    const width = el?.getBoundingClientRect().width ?? 176;
    const contentHeight = el?.scrollHeight ?? 220;
    const heightForPositioning = Math.min(contentHeight, maxHeight);
    positionMenu(anchor, width, heightForPositioning);
  }

  /** 采集菜单布局诊断信息，并写入后端日志（用于 Windows WebView2 环境排查）。 */
  async function logMenuDiagnostics(stage: "pre" | "post"): Promise<void> {
    if (!isTauriRuntime()) return;
    try {
      const anchorRect = anchorEl?.getBoundingClientRect() ?? null;
      const menuRect = menuEl?.getBoundingClientRect() ?? null;
      const cs = menuEl ? window.getComputedStyle(menuEl) : null;
      const buttons = menuEl ? Array.from(menuEl.querySelectorAll("button")) : [];
      const payload = {
        stage,
        time: new Date().toISOString(),
        devicePixelRatio: window.devicePixelRatio,
        inner: { w: window.innerWidth, h: window.innerHeight },
        open,
        menuX,
        menuY,
        maxHeightPx,
        anchorRect,
        menuRect,
        menu: menuEl
          ? {
              childButtons: menuEl.querySelectorAll("button").length,
              buttonRects: buttons.map((btn: HTMLButtonElement) => {
                const bcs = window.getComputedStyle(btn);
                return {
                  text: btn.textContent?.trim() ?? "",
                  rect: btn.getBoundingClientRect(),
                  display: bcs.display,
                  width: bcs.width,
                  paddingY: `${bcs.paddingTop}/${bcs.paddingBottom}`,
                };
              }),
              scrollHeight: menuEl.scrollHeight,
              clientHeight: menuEl.clientHeight,
              offsetHeight: menuEl.offsetHeight,
              overflowY: cs?.overflowY ?? null,
              maxHeight: cs?.maxHeight ?? null,
            }
          : null,
      };
      await frontendLog("info", `[more_menu_diag] ${JSON.stringify(payload)}`);
    } catch (e) {
      console.warn("[more_menu_diag_error]", e);
    }
  }

  /** 打开右上角菜单。 */
  async function openMenu(e: MouseEvent): Promise<void> {
    const anchor = e.currentTarget;
    if (!(anchor instanceof HTMLElement)) return;
    anchorEl = anchor;

    updateMenuLayout(anchor, null);
    open = true;

    if (e.shiftKey) {
      await logMenuDiagnostics("pre");
    }

    await tick();
    if (!open || !menuEl || !anchorEl) return;
    updateMenuLayout(anchorEl, menuEl);

    if (e.shiftKey) {
      await logMenuDiagnostics("post");
    }
  }

  /** 关闭右上角菜单。 */
  function closeMenu(): void {
    open = false;
  }

  /** 切换菜单显示状态。 */
  function toggleMenu(e: MouseEvent): void {
    if (open) {
      closeMenu();
      return;
    }
    void openMenu(e);
  }

  /** 当菜单打开时，监听窗口尺寸变化并重新定位，避免 DPI/窗口变化导致裁剪。 */
  function onPositionEffect(): (() => void) | void {
    if (!open || !anchorEl || !menuEl) return;

    /** 处理窗口尺寸变化：使用当前真实尺寸重新定位菜单。 */
    function onResize(): void {
      if (!anchorEl || !menuEl) return;
      updateMenuLayout(anchorEl, menuEl);
    }

    window.addEventListener("resize", onResize);
    return (): void => {
      window.removeEventListener("resize", onResize);
    };
  }

  $effect(onPositionEffect);

  /** 点击“管理黑名单”。 */
  function onBlacklistClick(): void {
    closeMenu();
    props.onOpenBlacklist();
  }

  /** 点击“设置”。 */
  function onSettingsClick(): void {
    closeMenu();
    props.onOpenSettings();
  }

  /** 点击“窗口置顶/取消置顶”。 */
  function onAlwaysOnTopClick(): void {
    closeMenu();
    props.onToggleAlwaysOnTop();
  }

  /** 点击“迷你模式/退出迷你模式”。 */
  function onMiniModeClick(): void {
    closeMenu();
    props.onToggleMiniMode();
  }
</script>

<div class="relative">
  <button
    class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm whitespace-nowrap shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggleMenu}
  >
    更多
  </button>
  {#if open}
    <button class="fixed inset-0 z-40" type="button" aria-label="关闭菜单" onclick={closeMenu}></button>
    <div
      bind:this={menuEl}
      class="fixed z-50 flex w-44 flex-col gap-1 overflow-x-hidden overflow-y-auto rounded-2xl border border-black/10 bg-white p-1 shadow-2xl dark:border-white/10 dark:bg-zinc-900"
      style={`left:${menuX}px; top:${menuY}px; max-height:${maxHeightPx}px;`}
    >
      <button
        class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
        onclick={onBlacklistClick}
      >
        管理黑名单
      </button>
      <button
        class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
        onclick={onSettingsClick}
      >
        设置
      </button>
      <button
        class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
        onclick={onAlwaysOnTopClick}
      >
        {props.alwaysOnTop ? "取消置顶" : "窗口置顶"}
      </button>
      <button
        class="block w-full rounded-xl px-4 py-3 text-left text-sm text-zinc-900 hover:bg-black/5 dark:text-zinc-50 dark:hover:bg-white/10"
        onclick={onMiniModeClick}
      >
        {props.miniMode ? "退出迷你模式" : "迷你模式"}
      </button>
    </div>
  {/if}
</div>
