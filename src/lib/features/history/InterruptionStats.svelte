<script lang="ts">
  import { formatMmSs } from "$lib/utils/time";
  import type { InterruptionReasonCount, InterruptionStats } from "$lib/shared/types";

  const props = $props<{ stats: InterruptionStats | null; loading: boolean; error: string | null }>();

  const colors = ["#10b981", "#3b82f6", "#a855f7", "#f59e0b", "#ef4444", "#14b8a6", "#22c55e", "#60a5fa"];

  /** 将浮点数转为百分比字符串（保留 1 位小数）。 */
  function percent(v: number): string {
    return `${(v * 100).toFixed(1)}%`;
  }

  /** 计算数组最大值（空数组返回 0）。 */
  function max(arr: number[]): number {
    let m = 0;
    for (const n of arr) m = Math.max(m, n);
    return m;
  }

  /** 生成饼图切片 path（SVG）。 */
  function pieSlices(
    items: InterruptionReasonCount[],
  ): Array<{ d: string; color: string; label: string; count: number }> {
    const total = items.reduce((acc, it) => acc + it.count, 0);
    if (total <= 0) return [];

    const cx = 48;
    const cy = 48;
    const r = 42;
    let start = -Math.PI / 2;

    /** 极坐标转直角坐标。 */
    function point(angle: number): { x: number; y: number } {
      return { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
    }

    const out: Array<{ d: string; color: string; label: string; count: number }> = [];
    items.forEach((it, idx) => {
      const p = it.count / total;
      const end = start + p * Math.PI * 2;
      const large = end - start > Math.PI ? 1 : 0;
      const a = point(start);
      const b = point(end);
      const d = `M ${cx} ${cy} L ${a.x} ${a.y} A ${r} ${r} 0 ${large} 1 ${b.x} ${b.y} Z`;
      out.push({ d, color: colors[idx % colors.length] ?? "#94a3b8", label: it.reason, count: it.count });
      start = end;
    });
    return out;
  }
</script>

{#if props.loading}
  <div class="text-sm text-zinc-500 dark:text-zinc-400">正在生成中断分析...</div>
{:else if props.error}
  <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">分析失败：{props.error}</div>
{:else if !props.stats}
  <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无中断数据</div>
{:else}
  <div class="space-y-4">
    <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
      <div class="rounded-2xl bg-black/5 p-3 text-sm dark:bg-white/10">
        <div class="text-xs text-zinc-600 dark:text-zinc-300">中断总次数</div>
        <div class="mt-1 text-xl font-semibold text-zinc-900 tabular-nums dark:text-zinc-50">
          {props.stats.totalInterruptions}
        </div>
        <div class="mt-2 text-xs text-zinc-600 dark:text-zinc-300">
          每日平均：{props.stats.dailyAverage.toFixed(2)} · 每周平均：{props.stats.weeklyAverage.toFixed(2)}
        </div>
      </div>
      <div class="rounded-2xl bg-black/5 p-3 text-sm dark:bg-white/10">
        <div class="text-xs text-zinc-600 dark:text-zinc-300">中断率 / 平均专注时长</div>
        <div class="mt-1 flex items-end justify-between gap-3">
          <div class="text-xl font-semibold text-zinc-900 tabular-nums dark:text-zinc-50">
            {percent(props.stats.interruptionRate)}
          </div>
          <div class="text-sm text-zinc-700 tabular-nums dark:text-zinc-200">
            {formatMmSs(Math.round(props.stats.averageFocusedSeconds))}
          </div>
        </div>
        <div class="mt-2 text-xs text-zinc-600 dark:text-zinc-300">中断率 = 中断番茄数 /（完成 + 中断）</div>
      </div>
    </div>

    <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
      <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">中断时段（按小时）</div>
      <div class="grid items-end gap-0.5" style="grid-template-columns: repeat(24, minmax(0, 1fr));">
        {#each props.stats.hourlyCounts as v, i (i)}
          <div class="flex flex-col items-center gap-1">
            <div
              class="w-2 rounded bg-red-500/70 dark:bg-red-400/60"
              style={`height:${max(props.stats.hourlyCounts) ? Math.max(4, Math.round((v / max(props.stats.hourlyCounts)) * 56)) : 4}px`}
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

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">原因分布</div>
        {#if props.stats.reasonDistribution.length === 0}
          <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无</div>
        {:else}
          <div class="flex items-center gap-4">
            <svg viewBox="0 0 96 96" class="h-28 w-28 shrink-0">
              {#each pieSlices(props.stats.reasonDistribution) as s (s.label)}
                <path d={s.d} fill={s.color}></path>
              {/each}
            </svg>
            <div class="min-w-0 flex-1 space-y-2">
              {#each props.stats.reasonDistribution as it, idx (it.reason)}
                <div class="flex items-center justify-between gap-2 text-sm">
                  <div class="flex min-w-0 items-center gap-2">
                    <span class="h-2 w-2 rounded-full" style={`background:${colors[idx % colors.length]};`}></span>
                    <span class="truncate text-zinc-800 dark:text-zinc-100">{it.reason}</span>
                  </div>
                  <span class="text-zinc-700 tabular-nums dark:text-zinc-200">{it.count}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
        <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">解读</div>
        <div class="space-y-2 text-sm text-zinc-700 dark:text-zinc-200">
          <div>中断越集中在某些小时，越可能是固定干扰源（会议/消息/疲劳）。</div>
          <div>原因分布可帮助你为高频原因制定“预案”。</div>
          <div>平均专注时长可用来评估当前番茄时长是否过长。</div>
        </div>
      </div>
    </div>
  </div>
{/if}
