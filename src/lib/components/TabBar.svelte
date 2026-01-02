<script lang="ts">
  import { page } from "$app/stores";

  type TabKey = "timer" | "history" | "settings";

  type TabItem = {
    /** tab 唯一标识。 */
    key: TabKey;
    /** 展示文案。 */
    label: string;
    /** 图标（使用 emoji，后续可替换为 icon font/svg）。 */
    icon: string;
    /** 目标路由。 */
    href: string;
  };

  const tabs: TabItem[] = [
    { key: "timer", label: "计时", icon: "⏱️", href: "/" },
    { key: "history", label: "历史", icon: "📊", href: "/history" },
    { key: "settings", label: "设置", icon: "⚙️", href: "/settings" },
  ];

  /** 将 pathname 归一化为 tab key（用于高亮选中态）。 */
  function tabFromPathname(pathname: string): TabKey {
    if (pathname.startsWith("/history")) return "history";
    if (pathname.startsWith("/settings")) return "settings";
    return "timer";
  }

  /** 当前选中的 tab。 */
  const activeTab = $derived(tabFromPathname($page.url.pathname));
</script>

<nav
  class="fixed inset-x-0 bottom-0 z-40 border-t border-black/10 bg-white shadow-sm dark:border-white/10 dark:bg-zinc-900"
  aria-label="底部导航"
>
  <div class="mx-auto grid max-w-md grid-cols-3 px-2 pt-2 pb-[max(0px,env(safe-area-inset-bottom))]">
    {#each tabs as t (t.key)}
      <a
        href={t.href}
        class={"flex flex-col items-center justify-center gap-1 rounded-2xl px-2 py-2 text-xs " +
          (activeTab === t.key
            ? "text-zinc-900 dark:text-zinc-50"
            : "text-zinc-400 hover:bg-black/5 dark:text-zinc-500 dark:hover:bg-white/10")}
        aria-current={activeTab === t.key ? "page" : undefined}
      >
        <div class={activeTab === t.key ? "text-base" : "text-base opacity-80"} aria-hidden="true">{t.icon}</div>
        <div class={activeTab === t.key ? "font-medium" : ""}>{t.label}</div>
      </a>
    {/each}
  </div>
</nav>
