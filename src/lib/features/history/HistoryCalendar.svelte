<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const props = $props<{ counts: Record<string, number>; selectedDate?: string }>();

  const dispatch = createEventDispatcher<{
    select: string;
  }>();

  const weekdays: string[] = ["一", "二", "三", "四", "五", "六", "日"];

  /** 将 Date 格式化为 `YYYY-MM-DD`。 */
  function formatYmd(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, "0");
    const d = String(date.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }

  /** 解析 `YYYY-MM-DD` 为 Date（本地时区，00:00）。 */
  function parseYmd(ymd: string): Date {
    const [y, m, d] = ymd.split("-").map((x) => Number(x));
    const dt = new Date();
    dt.setFullYear(y ?? 1970);
    dt.setMonth(Math.max(0, (m ?? 1) - 1));
    dt.setDate(d ?? 1);
    dt.setHours(0, 0, 0, 0);
    return dt;
  }

  /** 获取某个 Date 所在周的周一（本地时区，00:00）。 */
  function mondayOfWeek(date: Date): Date {
    const dt = new Date(date);
    dt.setHours(0, 0, 0, 0);
    const day = dt.getDay(); // 0=Sun..6=Sat
    const offset = (day + 6) % 7; // Mon=0..Sun=6
    dt.setDate(dt.getDate() - offset);
    return dt;
  }

  /** 生成最近 N 周（按周一对齐）的日期矩阵：weeks[weekIndex][weekdayIndex]。 */
  function lastNWeeksAligned(weekCount: number): string[][] {
    const end = new Date();
    end.setHours(0, 0, 0, 0);
    const endMonday = mondayOfWeek(end);
    const startMonday = new Date(endMonday);
    startMonday.setDate(endMonday.getDate() - (weekCount - 1) * 7);

    const weeks: string[][] = [];
    for (let w = 0; w < weekCount; w += 1) {
      const week: string[] = [];
      for (let d = 0; d < 7; d += 1) {
        const dt = new Date(startMonday);
        dt.setDate(startMonday.getDate() + w * 7 + d);
        week.push(formatYmd(dt));
      }
      weeks.push(week);
    }
    return weeks;
  }

  /** 将日期映射为 `M月` 文案。 */
  function monthLabel(ymd: string): string {
    const dt = parseYmd(ymd);
    return `${dt.getMonth() + 1}月`;
  }

  /** 判断某个周列是否应该显示“月份标签”（遇到跨月或包含当月 1 号）。 */
  function monthHeaderForWeek(week: string[], prevWeek: string[] | null): string | null {
    const hasFirst = week.some((d) => d.endsWith("-01"));
    if (hasFirst) {
      const first = week.find((d) => d.endsWith("-01")) ?? week[0];
      return first ? monthLabel(first) : null;
    }
    const cur = week[0] ? monthLabel(week[0]) : null;
    const prev = prevWeek?.[0] ? monthLabel(prevWeek[0]) : null;
    if (cur && prev && cur !== prev) return cur;
    return null;
  }

  let weeks = $state<string[][]>(lastNWeeksAligned(12));
  let hoverDate = $state<string | null>(null);

  /** 计算热力颜色 class（按完成数量分级）。 */
  function cellClass(count: number): string {
    if (count <= 0) return "bg-black/5 dark:bg-white/10";
    if (count <= 2) return "bg-emerald-300/70 dark:bg-emerald-400/40";
    if (count <= 4) return "bg-emerald-400/80 dark:bg-emerald-400/60";
    return "bg-emerald-500/90 dark:bg-emerald-500/70";
  }

  /** 将完成数量转为友好的中文文案。 */
  function countText(count: number): string {
    return `${count} 个`;
  }

  /** 点击某个日期。 */
  function onPick(date: string): void {
    dispatch("select", date);
  }

  /** 鼠标悬停/聚焦某个日期时，记录悬停日期（用于展示具体日期）。 */
  function onHover(date: string): void {
    hoverDate = date;
  }

  /** 鼠标移出/失焦时清理悬停日期。 */
  function onHoverEnd(date: string): void {
    if (hoverDate === date) hoverDate = null;
  }
</script>

<div class="space-y-2">
  <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-zinc-600 dark:text-zinc-300">
    <div>
      已选：<span class="font-medium text-zinc-900 tabular-nums dark:text-zinc-50">{props.selectedDate ?? "—"}</span>
      {#if props.selectedDate}
        · {countText(props.counts[props.selectedDate] ?? 0)}
      {/if}
    </div>
    <div>
      悬停：<span class="font-medium text-zinc-900 tabular-nums dark:text-zinc-50">{hoverDate ?? "—"}</span>
      {#if hoverDate}
        · {countText(props.counts[hoverDate] ?? 0)}
      {/if}
    </div>
  </div>

  <div class="flex gap-2">
    <div class="pt-4">
      <div class="grid grid-rows-7 gap-1">
        {#each weekdays as w (w)}
          <div class="flex h-4 items-center justify-end pr-1 text-[10px] text-zinc-500 dark:text-zinc-400">{w}</div>
        {/each}
      </div>
    </div>

    <div class="min-w-0">
      <div class="grid grid-cols-12 gap-1 text-[10px] text-zinc-500 dark:text-zinc-400">
        {#each weeks as week, wi (wi)}
          {@const label = monthHeaderForWeek(week, wi > 0 ? (weeks[wi - 1] ?? null) : null)}
          <div class="h-4 leading-4">{label ?? ""}</div>
        {/each}
      </div>

      <div class="grid grid-cols-12 gap-1">
        {#each weeks as week, wi (wi)}
          <div class="grid grid-rows-7 gap-1">
            {#each week as date (date)}
              {@const count = props.counts[date] ?? 0}
              <button
                type="button"
                class={"h-4 w-4 rounded " +
                  cellClass(count) +
                  (props.selectedDate === date ? " ring-2 ring-zinc-900 dark:ring-white" : "")}
                title={`${date}：${countText(count)}`}
                aria-label={`${date}：完成 ${countText(count)}`}
                onclick={() => onPick(date)}
                onpointerenter={() => onHover(date)}
                onpointerleave={() => onHoverEnd(date)}
                onfocus={() => onHover(date)}
                onblur={() => onHoverEnd(date)}
              ></button>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="flex items-center gap-2 text-[10px] text-zinc-500 dark:text-zinc-400">
    <span>少</span>
    <div class="flex items-center gap-1">
      <span class={"h-3 w-3 rounded " + cellClass(0)}></span>
      <span class={"h-3 w-3 rounded " + cellClass(1)}></span>
      <span class={"h-3 w-3 rounded " + cellClass(3)}></span>
      <span class={"h-3 w-3 rounded " + cellClass(5)}></span>
    </div>
    <span>多</span>
  </div>
</div>
