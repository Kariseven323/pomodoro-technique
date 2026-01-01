<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const props = $props<{ counts: Record<string, number>; selectedDate?: string }>();

  const dispatch = createEventDispatcher<{
    select: string;
  }>();

  /** 将 Date 格式化为 `YYYY-MM-DD`。 */
  function formatYmd(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, "0");
    const d = String(date.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }

  /** 生成最近 N 天日期（从旧到新）。 */
  function lastNDays(n: number): string[] {
    const out: string[] = [];
    const end = new Date();
    end.setHours(0, 0, 0, 0);
    for (let i = n - 1; i >= 0; i -= 1) {
      const dt = new Date(end);
      dt.setDate(end.getDate() - i);
      out.push(formatYmd(dt));
    }
    return out;
  }

  let dates = $state<string[]>(lastNDays(84));

  /** 将数组按固定长度分组。 */
  function chunk<T>(items: T[], size: number): T[][] {
    const out: T[][] = [];
    for (let i = 0; i < items.length; i += size) {
      out.push(items.slice(i, i + size));
    }
    return out;
  }

  let weeks = $state<string[][]>(chunk(dates, 7));

  /** 计算热力颜色 class（按完成数量分级）。 */
  function cellClass(count: number): string {
    if (count <= 0) return "bg-black/5 dark:bg-white/10";
    if (count <= 2) return "bg-emerald-300/70 dark:bg-emerald-400/40";
    if (count <= 4) return "bg-emerald-400/80 dark:bg-emerald-400/60";
    return "bg-emerald-500/90 dark:bg-emerald-500/70";
  }

  /** 点击某个日期。 */
  function onPick(date: string): void {
    dispatch("select", date);
  }
</script>

<div class="grid grid-cols-12 gap-1">
  {#each weeks as week, wi (wi)}
    <div class="grid grid-rows-7 gap-1">
      {#each week as date (date)}
        <button
          type="button"
          class={
            "h-4 w-4 rounded " +
            cellClass(props.counts[date] ?? 0) +
            (props.selectedDate === date ? " ring-2 ring-zinc-900 dark:ring-white" : "")
          }
          title={`${date}：${props.counts[date] ?? 0} 个`}
          onclick={() => onPick(date)}
        ></button>
      {/each}
    </div>
  {/each}
</div>
