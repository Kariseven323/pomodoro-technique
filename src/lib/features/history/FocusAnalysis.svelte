<script lang="ts">
  import type { FocusAnalysis } from "$lib/shared/types";

  const props = $props<{ analysis: FocusAnalysis | null; loading: boolean; error: string | null }>();

  type HeatHover = { weekdayIndex: number; hour: number; value: number };

  /** 计算数组最大值（空数组返回 0）。 */
  function max(arr: number[]): number {
    let m = 0;
    for (const n of arr) m = Math.max(m, n);
    return m;
  }

  /** 将值映射为热力强度 class。 */
  function heatClass(v: number, maxV: number): string {
    if (maxV <= 0 || v <= 0) return "bg-black/5 dark:bg-white/10";
    const p = v / maxV;
    if (p < 0.25) return "bg-sky-300/60 dark:bg-sky-400/30";
    if (p < 0.5) return "bg-sky-400/70 dark:bg-sky-400/45";
    if (p < 0.75) return "bg-sky-500/70 dark:bg-sky-500/55";
    return "bg-sky-600/80 dark:bg-sky-600/60";
  }

  /** 将星期索引映射为中文。 */
  function weekdayLabel(i: number): string {
    return ["一", "二", "三", "四", "五", "六", "日"][i] ?? "?";
  }

  /** 计算二维矩阵最大值（空矩阵返回 0）。 */
  function maxMatrix(matrix: number[][]): number {
    let m = 0;
    for (const row of matrix) {
      for (const v of row) m = Math.max(m, v);
    }
    return m;
  }

  const periodItems: Array<[string, number]> = [
    ["0-6", 0],
    ["6-12", 1],
    ["12-18", 2],
    ["18-24", 3],
  ];

  const hourTicks: number[] = [0, 6, 12, 18, 23];

  /** 周/小时热力图的最大值（用于强度归一化）。 */
  let heatMax = $derived(props.analysis ? maxMatrix(props.analysis.weekdayHourCounts) : 0);
  let heatHover = $state<HeatHover | null>(null);

  /** 生成周/小时热力图的悬停提示文案。 */
  function heatHoverText(hover: HeatHover | null): string {
    if (!hover) return "周— · —点：—";
    return `周${weekdayLabel(hover.weekdayIndex)} · ${hover.hour}点：${hover.value}`;
  }

  /** 设置热力图悬停状态（鼠标/键盘聚焦）。 */
  function onHeatHover(weekdayIndex: number, hour: number, value: number): void {
    heatHover = { weekdayIndex, hour, value };
  }

  /** 清理热力图悬停状态（鼠标离开整个热力图/焦点离开热力图时触发）。 */
  function clearHeatHover(): void {
    heatHover = null;
  }

  /** 当焦点离开热力图容器时清理悬停状态（避免在格子间切换导致反复闪动）。 */
  function onHeatmapFocusOut(e: FocusEvent): void {
    const next = e.relatedTarget;
    if (!(next instanceof Node)) {
      clearHeatHover();
      return;
    }
    if (!e.currentTarget) return;
    if (e.currentTarget instanceof HTMLElement && !e.currentTarget.contains(next)) clearHeatHover();
  }
</script>

{#if props.loading}
  <div class="text-sm text-zinc-500 dark:text-zinc-400">正在生成分析...</div>
{:else if props.error}
  <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">分析失败：{props.error}</div>
{:else if !props.analysis}
  <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无分析数据</div>
{:else}
  <div class="space-y-4">
    <div class="rounded-2xl bg-black/5 p-3 text-sm text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
      {props.analysis.summary}
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">小时分布</div>
        <div class="grid items-end gap-0.5" style="grid-template-columns: repeat(24, minmax(0, 1fr));">
          {#each props.analysis.hourlyCounts as v, i (i)}
            <div class="flex flex-col items-center gap-1">
              <div
                class="w-2 rounded bg-emerald-500/70 dark:bg-emerald-400/60"
                style={`height:${max(props.analysis.hourlyCounts) ? Math.max(4, Math.round((v / max(props.analysis.hourlyCounts)) * 56)) : 4}px`}
                title={`${i} 点：${v}`}
              ></div>
            </div>
          {/each}
        </div>
        <div class="mt-2 flex justify-between text-[10px] text-zinc-500 dark:text-zinc-400">
          <span>0</span>
          <span>6</span>
          <span>12</span>
          <span>18</span>
          <span>23</span>
        </div>
      </div>

      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">时段分布</div>
        <div class="space-y-2">
          {#each periodItems as [label, idx] (label)}
            {@const v = props.analysis.periodCounts[idx] ?? 0}
            <div class="flex items-center gap-2">
              <div class="w-12 text-xs text-zinc-600 dark:text-zinc-300">{label}</div>
              <div class="h-2 flex-1 overflow-hidden rounded-full bg-black/5 dark:bg-white/10">
                <div
                  class="h-full rounded-full bg-sky-500/70"
                  style={`width:${max(props.analysis.periodCounts) ? Math.round((v / max(props.analysis.periodCounts)) * 100) : 0}%`}
                ></div>
              </div>
              <div class="w-8 text-right text-xs text-zinc-600 tabular-nums dark:text-zinc-300">{v}</div>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">星期分布</div>
        <div class="space-y-2">
          {#each props.analysis.weekdayCounts as v, i (i)}
            <div class="flex items-center gap-2">
              <div class="w-8 text-xs text-zinc-600 dark:text-zinc-300">周{weekdayLabel(i)}</div>
              <div class="h-2 flex-1 overflow-hidden rounded-full bg-black/5 dark:bg-white/10">
                <div
                  class="h-full rounded-full bg-emerald-500/70"
                  style={`width:${max(props.analysis.weekdayCounts) ? Math.round((v / max(props.analysis.weekdayCounts)) * 100) : 0}%`}
                ></div>
              </div>
              <div class="w-8 text-right text-xs text-zinc-600 tabular-nums dark:text-zinc-300">{v}</div>
            </div>
          {/each}
        </div>
      </div>

      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">标签效率（平均分钟）</div>
        {#if props.analysis.tagEfficiency.length === 0}
          <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无</div>
        {:else}
          <div class="max-h-40 space-y-2 overflow-auto pr-1">
            {#each props.analysis.tagEfficiency as t (t.tag)}
              <div class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10">
                <div class="min-w-0 flex-1 truncate">{t.tag}</div>
                <div class="ml-2 text-zinc-700 tabular-nums dark:text-zinc-200">{t.avgDuration.toFixed(1)}</div>
                <div class="ml-2 text-xs text-zinc-500 tabular-nums dark:text-zinc-400">({t.count})</div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
      <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">周/小时热力图</div>
      <div class="mb-2 flex items-center justify-between gap-2 text-xs text-zinc-600 dark:text-zinc-300">
        <div class="shrink-0">纵轴：星期 · 横轴：小时（0-23）</div>
        <div class="min-h-[14px] shrink-0 font-medium whitespace-nowrap text-zinc-800 tabular-nums dark:text-zinc-100">
          {heatHoverText(heatHover)}
        </div>
      </div>
      <div class="overflow-auto">
        <div class="inline-block" onpointerleave={clearHeatHover} onfocusout={onHeatmapFocusOut}>
          <div class="grid grid-cols-[28px_repeat(24,10px)] gap-1">
            {#each Array.from({ length: 7 }) as _, wi}
              <div class="flex items-center justify-center text-[10px] text-zinc-500 dark:text-zinc-400">
                周{weekdayLabel(wi)}
              </div>
              {#each props.analysis.weekdayHourCounts[wi] ?? Array.from({ length: 24 }).map(() => 0) as v, hi (hi)}
                <button
                  type="button"
                  class={"h-3 w-3 rounded focus:outline-none " + heatClass(v, heatMax)}
                  title={`周${weekdayLabel(wi)} ${hi}点：${v}`}
                  aria-label={`周${weekdayLabel(wi)} ${hi}点：${v}`}
                  onpointerenter={() => onHeatHover(wi, hi, v)}
                  onfocus={() => onHeatHover(wi, hi, v)}
                ></button>
              {/each}
            {/each}
          </div>

          <div class="mt-2 pl-[28px]">
            <div class="grid grid-cols-[repeat(24,10px)] gap-1 text-[10px] text-zinc-500 dark:text-zinc-400">
              {#each hourTicks as h (h)}
                <div class="leading-none" style={`grid-column:${h + 1};`}>{h}</div>
              {/each}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
