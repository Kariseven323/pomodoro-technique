<script lang="ts">
  import { onMount } from "svelte";
  import { dev } from "$app/environment";
  import { appData, historyDevChangedAt, timerSnapshot } from "$lib/stores/appClient";
  import ExportModal from "$lib/features/history/ExportModal.svelte";
  import FocusAnalysisView from "$lib/features/history/FocusAnalysis.svelte";
  import InterruptionStatsView from "$lib/features/history/InterruptionStats.svelte";
  import HistoryCalendar from "$lib/features/history/HistoryCalendar.svelte";
  import {
    exportHistory,
    getFocusAnalysis,
    getHistory,
    getInterruptionStats,
    setHistoryRemark,
    setMiniMode,
  } from "$lib/api/tauri";
  import { miniMode } from "$lib/stores/uiState";
  import type {
    DateRange,
    ExportField,
    ExportFormat,
    ExportRequest,
    FocusAnalysis,
    HistoryDay,
    HistoryRecord,
    InterruptionDay,
    InterruptionRecord,
    InterruptionStats,
  } from "$lib/shared/types";
  import { addDays, endOfMonthYmd, startOfMonthYmd, startOfWeekYmd, todayYmd } from "$lib/utils/date";
  import { formatMmSs } from "$lib/utils/time";

  type ViewMode = "day" | "week" | "month";
  type CombinedDay = { date: string; records: HistoryRecord[]; interruptions: InterruptionRecord[] };

  let viewMode = $state<ViewMode>("week");
  let selectedDate = $state<string>(todayYmd());
  let selectedMonth = $state<string>(todayYmd().slice(0, 7));

  let days = $state<HistoryDay[]>([]);
  let range = $state<DateRange>({ from: startOfWeekYmd(todayYmd()), to: todayYmd() });
  let loading = $state(false);
  let historyError = $state<string | null>(null);
  let loadingMore = $state(false);
  let hasMore = $state(true);

  let exportOpen = $state(false);
  let lastExportPath = $state<string | null>(null);
  let exportError = $state<string | null>(null);

  let analysis = $state<FocusAnalysis | null>(null);
  let analysisLoading = $state(false);
  let analysisError = $state<string | null>(null);

  let interruptionStats = $state<InterruptionStats | null>(null);
  let interruptionLoading = $state(false);
  let interruptionError = $state<string | null>(null);

  let remarkDrafts = $state<Record<string, string>>({});
  let remarkSavingKey = $state<string | null>(null);
  let expandedDates = $state<Set<string>>(new Set());

  /** 计算日历热力图映射：date -> count。 */
  function buildCounts(items: HistoryDay[]): Record<string, number> {
    const out: Record<string, number> = {};
    for (const d of items) {
      out[d.date] = d.records.length;
    }
    return out;
  }

  /** 在当前 range 内筛选中断记录（按日分组）。 */
  function interruptionDaysInRange(all: InterruptionDay[], r: DateRange): InterruptionDay[] {
    return all.filter((d) => d.date >= r.from && d.date <= r.to);
  }

  /** 合并“完成记录 + 中断记录”为统一的按日结构（用于历史列表展示）。 */
  function mergeDays(historyDays: HistoryDay[], interruptionDays: InterruptionDay[]): CombinedDay[] {
    const map = new Map<string, CombinedDay>();
    for (const d of historyDays) {
      map.set(d.date, { date: d.date, records: d.records, interruptions: [] });
    }
    for (const d of interruptionDays) {
      const prev = map.get(d.date);
      if (prev) {
        prev.interruptions = d.records;
      } else {
        map.set(d.date, { date: d.date, records: [], interruptions: d.records });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.date.localeCompare(a.date));
  }

  /** 从 ISO 时间戳提取本地 `HH:mm`（失败则回退为 `--:--`）。 */
  function hhmmFromIso(ts: string): string {
    try {
      const dt = new Date(ts);
      const h = String(dt.getHours()).padStart(2, "0");
      const m = String(dt.getMinutes()).padStart(2, "0");
      return `${h}:${m}`;
    } catch {
      return "--:--";
    }
  }

  /** 在工作阶段中断类型映射为更友好的中文文案。 */
  function interruptionTypeText(t: InterruptionRecord["type"]): string {
    if (t === "reset") return "重置";
    if (t === "skip") return "跳过";
    return "退出";
  }

  /** 当前展示用的“合并日列表”（历史 + 中断）。 */
  let combinedDays = $derived(mergeDays(days, interruptionDaysInRange($appData?.interruptions ?? [], range)));

  /** 进入导出：打开弹窗并清理上次提示。 */
  function openExport(): void {
    exportError = null;
    lastExportPath = null;
    exportOpen = true;
  }

  /** 关闭导出弹窗。 */
  function closeExport(): void {
    exportOpen = false;
  }

  /** 切换迷你模式。 */
  async function toggleMiniMode(): Promise<void> {
    const next = !$miniMode;
    await setMiniMode(next);
    miniMode.set(next);
  }

  /** 刷新历史列表（按当前 viewMode/range）。 */
  async function refreshHistory(): Promise<void> {
    loading = true;
    historyError = null;
    try {
      const res = await getHistory(range);
      days = res;
      hasMore = viewMode === "week";
      remarkDrafts = {};
      expandedDates = viewMode === "day" ? new Set([range.from]) : new Set();
    } catch (e) {
      historyError = e instanceof Error ? e.message : String(e);
      days = [];
    } finally {
      loading = false;
    }
  }

  /** 向前滚动加载更多（仅 week 视图）。 */
  async function loadMore(): Promise<void> {
    if (loadingMore || !hasMore) return;
    loadingMore = true;
    try {
      const newTo = addDays(range.from, -1);
      const newFrom = addDays(range.from, -7);
      const more = await getHistory({ from: newFrom, to: newTo });
      if (more.length === 0) {
        hasMore = false;
      } else {
        const seen = new Set(days.map((d) => d.date));
        const merged = [...days];
        for (const d of more) {
          if (!seen.has(d.date)) merged.push(d);
        }
        days = merged;
        range = { from: newFrom, to: range.to };
      }
    } finally {
      loadingMore = false;
    }
  }

  /** 处理滚动：接近底部时自动加载更多。 */
  function onScroll(e: Event): void {
    if (viewMode !== "week") return;
    const el = e.currentTarget as HTMLDivElement;
    const nearBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 120;
    if (nearBottom) void loadMore();
  }

  /** 切换日/周/月视图，并同步 range。 */
  function setMode(next: ViewMode): void {
    viewMode = next;
    if (next === "day") {
      range = { from: selectedDate, to: selectedDate };
    } else if (next === "week") {
      range = { from: startOfWeekYmd(todayYmd()), to: todayYmd() };
    } else {
      range = { from: startOfMonthYmd(selectedMonth), to: endOfMonthYmd(selectedMonth) };
    }
    void refreshHistory();
    void refreshAnalysis();
    void refreshInterruptionStats();
  }

  /** 在日历上选择日期：切到 day 视图并加载。 */
  function onSelectDate(e: CustomEvent<string>): void {
    selectedDate = e.detail;
    viewMode = "day";
    range = { from: selectedDate, to: selectedDate };
    void refreshHistory();
    void refreshAnalysis();
    void refreshInterruptionStats();
  }

  /** 切换某一天的展开/收起状态。 */
  function toggleDay(date: string): void {
    const next = new Set(expandedDates);
    if (next.has(date)) next.delete(date);
    else next.add(date);
    expandedDates = next;
  }

  /** 计算当前列表的标签统计。 */
  function tagStats(items: HistoryDay[]): Array<{ tag: string; count: number }> {
    const map = new Map<string, number>();
    for (const d of items) {
      for (const r of d.records) {
        map.set(r.tag, (map.get(r.tag) ?? 0) + 1);
      }
    }
    return Array.from(map.entries())
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
  }

  /** 拉取专注分析。 */
  async function refreshAnalysis(): Promise<void> {
    analysisLoading = true;
    analysisError = null;
    try {
      analysis = await getFocusAnalysis(range);
    } catch (e) {
      analysisError = e instanceof Error ? e.message : String(e);
      analysis = null;
    } finally {
      analysisLoading = false;
    }
  }

  /** 拉取中断分析。 */
  async function refreshInterruptionStats(): Promise<void> {
    interruptionLoading = true;
    interruptionError = null;
    try {
      interruptionStats = await getInterruptionStats(range);
    } catch (e) {
      interruptionError = e instanceof Error ? e.message : String(e);
      interruptionStats = null;
    } finally {
      interruptionLoading = false;
    }
  }

  /** 导出历史（后端弹出保存对话框）。 */
  async function onExportSubmit(
    e: CustomEvent<{ format: ExportFormat; fields: ExportField[]; range: DateRange }>,
  ): Promise<void> {
    exportError = null;
    lastExportPath = null;
    try {
      const request: ExportRequest = { range: e.detail.range, format: e.detail.format, fields: e.detail.fields };
      lastExportPath = await exportHistory(request);
      exportOpen = false;
    } catch (err) {
      exportError = err instanceof Error ? err.message : String(err);
    }
  }

  /** 将阶段映射为更友好的中文文案。 */
  function phaseText(phase: HistoryRecord["phase"]): string {
    if (phase === "work") return "工作";
    if (phase === "shortBreak") return "短休息";
    return "长休息";
  }

  /** 生成备注草稿 key。 */
  function remarkKey(date: string, recordIndex: number): string {
    return `${date}#${recordIndex}`;
  }

  /** 推导结束时间（用于旧数据缺失 endTime 的展示）。 */
  function derivedEndTime(record: HistoryRecord): string {
    if (record.endTime) return record.endTime;
    const [hh, mm] = record.startTime.split(":").map((x) => Number(x));
    const total = (hh ?? 0) * 60 + (mm ?? 0) + record.duration;
    const endH = Math.floor((total / 60) % 24);
    const endM = total % 60;
    return `${String(endH).padStart(2, "0")}:${String(endM).padStart(2, "0")}`;
  }

  /** 获取备注输入框的展示值（优先草稿，其次原值）。 */
  function remarkValue(date: string, recordIndex: number, original: string): string {
    const key = remarkKey(date, recordIndex);
    return key in remarkDrafts ? remarkDrafts[key] : original;
  }

  /** 记录备注输入变化（仅写入本地草稿，不立即落盘）。 */
  function onRemarkInput(date: string, recordIndex: number, value: string): void {
    const key = remarkKey(date, recordIndex);
    remarkDrafts = { ...remarkDrafts, [key]: value };
  }

  /** 保存某条记录备注，并本地更新列表（点击保存时触发）。 */
  async function saveRemark(date: string, recordIndex: number): Promise<void> {
    const key = remarkKey(date, recordIndex);
    remarkSavingKey = key;
    try {
      const updated = await setHistoryRemark(date, recordIndex, (remarkDrafts[key] ?? "").trim());
      const next = days.map((d) => {
        if (d.date !== date) return d;
        const records = d.records.map((r, i) => (i === recordIndex ? updated : r));
        return { ...d, records };
      });
      days = next;
      const { [key]: _removed, ...rest } = remarkDrafts;
      remarkDrafts = rest;
    } finally {
      remarkSavingKey = null;
    }
  }

  /** Svelte 生命周期：首次进入页面时加载当前周 + 分析。 */
  function onMounted(): void {
    void refreshHistory();
    void refreshAnalysis();
    void refreshInterruptionStats();
  }

  onMount(onMounted);

  /** 响应调试历史数据变更事件：自动刷新历史列表与分析（满足 PRD v3）。 */
  function onHistoryDevChangedEffect(): void {
    if ($historyDevChangedAt <= 0) return;
    void refreshHistory();
    void refreshAnalysis();
    void refreshInterruptionStats();
  }

  $effect(onHistoryDevChangedEffect);
</script>

<main
  class="min-h-screen bg-gradient-to-b from-zinc-50 to-zinc-100 px-4 py-6 text-zinc-900 dark:from-zinc-950 dark:to-zinc-900 dark:text-zinc-50"
>
  <div class="mx-auto w-full max-w-5xl">
    <header class="mb-5 flex items-center justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">历史记录</h1>
        <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">日/周/月筛选 · 备注编辑 · 导出 · 专注分析</p>
      </div>
      <div class="flex items-center gap-2">
        <a
          class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
          href="/"
        >
          返回
        </a>
        <button
          class="rounded-2xl border border-black/10 bg-white/70 px-4 py-2 text-sm shadow-sm hover:bg-white dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
          onclick={() => void toggleMiniMode()}
          disabled={!$timerSnapshot}
        >
          {$miniMode ? "退出迷你" : "迷你模式"}
        </button>
        <button
          class="rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
          onclick={openExport}
          disabled={days.length === 0}
        >
          导出
        </button>
      </div>
    </header>

    {#if dev && ($appData?.historyDev?.length ?? 0) > 0}
      <div
        class="mb-4 rounded-3xl border border-amber-500/20 bg-amber-500/10 p-4 text-sm text-amber-700 dark:text-amber-300"
      >
        当前正在展示测试历史数据（`history_dev`）。清除测试数据后将恢复展示正式历史数据。
      </div>
    {/if}

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <div
        class="rounded-3xl border border-white/20 bg-white/70 p-4 shadow-xl backdrop-blur-xl lg:col-span-1 dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="mb-3 flex items-center justify-between gap-2">
          <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">视图</div>
          <div class="flex items-center gap-2">
            <button
              class="rounded-xl px-2 py-1 text-xs hover:bg-black/5 dark:hover:bg-white/10"
              onclick={() => setMode("day")}
            >
              日
            </button>
            <button
              class="rounded-xl px-2 py-1 text-xs hover:bg-black/5 dark:hover:bg-white/10"
              onclick={() => setMode("week")}
            >
              周
            </button>
            <button
              class="rounded-xl px-2 py-1 text-xs hover:bg-black/5 dark:hover:bg-white/10"
              onclick={() => setMode("month")}
            >
              月
            </button>
          </div>
        </div>

        {#if viewMode === "day"}
          <label class="block">
            <div class="mb-1 text-xs text-zinc-600 dark:text-zinc-300">选择日期</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="date"
              bind:value={selectedDate}
              onchange={() => setMode("day")}
            />
          </label>
        {:else if viewMode === "month"}
          <label class="block">
            <div class="mb-1 text-xs text-zinc-600 dark:text-zinc-300">选择月份</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="month"
              bind:value={selectedMonth}
              onchange={() => setMode("month")}
            />
          </label>
        {/if}

        <div class="mt-4">
          <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">日历热力图</div>
          <HistoryCalendar counts={buildCounts(days)} {selectedDate} on:select={onSelectDate} />
        </div>

        <div class="mt-4">
          <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">按标签统计</div>
          {#if days.length === 0}
            <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无</div>
          {:else}
            <div class="max-h-40 space-y-2 overflow-auto pr-1">
              {#each tagStats(days) as item (item.tag)}
                <div
                  class="flex items-center justify-between rounded-2xl bg-black/5 px-3 py-2 text-sm dark:bg-white/10"
                >
                  <div class="truncate">{item.tag}</div>
                  <div class="tabular-nums">{item.count}</div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="mt-4 rounded-2xl bg-black/5 p-3 text-xs text-zinc-600 dark:bg-white/10 dark:text-zinc-300">
          当前范围：{range.from} ~ {range.to}
        </div>

        {#if exportError}
          <div class="mt-3 rounded-2xl bg-red-500/10 p-3 text-xs text-red-600 dark:text-red-300">
            导出失败：{exportError}
          </div>
        {/if}
        {#if lastExportPath}
          <div class="mt-3 rounded-2xl bg-emerald-500/10 p-3 text-xs text-emerald-700 dark:text-emerald-300">
            已导出：{lastExportPath}
          </div>
        {/if}
      </div>

      <div
        class="rounded-3xl border border-white/20 bg-white/70 p-4 shadow-xl backdrop-blur-xl lg:col-span-2 dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="mb-3 flex items-center justify-between gap-2">
          <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">记录列表</div>
          <div class="text-xs text-zinc-600 dark:text-zinc-300">
            {#if loading}加载中...{:else}{combinedDays.length} 天{/if}
          </div>
        </div>

        <div class="max-h-[520px] space-y-3 overflow-auto pr-1" onscroll={onScroll}>
          {#if loading}
            <div class="text-sm text-zinc-500 dark:text-zinc-400">正在加载历史...</div>
          {:else if historyError}
            <div class="rounded-2xl bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300">
              加载失败：{historyError}
            </div>
          {:else if combinedDays.length === 0}
            <div class="text-sm text-zinc-500 dark:text-zinc-400">暂无历史记录</div>
          {:else}
            {#each combinedDays as d (d.date)}
              <div class="rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
                <button
                  type="button"
                  class="flex w-full items-center justify-between rounded-xl px-1 py-1 text-left hover:bg-black/5 dark:hover:bg-white/10"
                  onclick={() => toggleDay(d.date)}
                >
                  <div class="text-sm font-medium text-zinc-900 dark:text-zinc-50">{d.date}</div>
                  <div class="flex items-center gap-2 text-xs text-zinc-600 dark:text-zinc-300">
                    <span>完成 {d.records.length}</span>
                    <span>中断 {d.interruptions.length}</span>
                    <span>{expandedDates.has(d.date) ? "收起" : "展开"}</span>
                  </div>
                </button>
                {#if expandedDates.has(d.date)}
                  <div class="mt-3 space-y-2">
                    {#each d.records as r, i (r.startTime + i)}
                      <div class="rounded-2xl bg-black/5 p-3 text-sm dark:bg-white/10">
                        <div class="flex items-center justify-between gap-3">
                          <div class="min-w-0">
                            <div class="truncate font-medium text-zinc-900 dark:text-zinc-50">{r.tag}</div>
                            <div class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">
                              {r.startTime} - {derivedEndTime(r)} · {r.duration} 分钟 · {phaseText(r.phase)}
                            </div>
                          </div>
                        </div>
                        <div class="mt-2 flex items-center gap-2">
                          <input
                            class="min-w-0 flex-1 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
                            placeholder="备注（可选）"
                            value={remarkValue(d.date, i, r.remark)}
                            oninput={(ev) => onRemarkInput(d.date, i, (ev.currentTarget as HTMLInputElement).value)}
                          />
                          <button
                            class="shrink-0 rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-xs text-zinc-700 hover:bg-white disabled:opacity-40 dark:border-white/10 dark:bg-white/5 dark:text-zinc-200 dark:hover:bg-white/10"
                            disabled={remarkSavingKey === remarkKey(d.date, i) ||
                              remarkValue(d.date, i, r.remark).trim() === r.remark}
                            onclick={() => void saveRemark(d.date, i)}
                          >
                            {remarkSavingKey === remarkKey(d.date, i) ? "保存中" : "保存"}
                          </button>
                        </div>
                      </div>
                    {/each}

                    {#if d.interruptions.length > 0}
                      <div class="mt-3">
                        <div class="mb-2 text-xs font-medium text-zinc-600 dark:text-zinc-300">中断记录</div>
                        <div class="space-y-2">
                          {#each d.interruptions as it, ii (it.timestamp + ii)}
                            <div
                              class="rounded-2xl border border-red-500/20 bg-red-500/10 p-3 text-sm dark:border-red-500/20"
                            >
                              <div class="min-w-0">
                                <div class="truncate font-medium text-zinc-900 dark:text-zinc-50">
                                  中断 · {interruptionTypeText(it.type)}
                                </div>
                                <div class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">
                                  {hhmmFromIso(it.timestamp)} · 已专注 {formatMmSs(it.focusedSeconds)} · 剩余
                                  {formatMmSs(it.remainingSeconds)}
                                </div>
                                <div class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">标签：{it.tag}</div>
                                {#if it.reason.trim()}
                                  <div class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">原因：{it.reason}</div>
                                {/if}
                              </div>
                            </div>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}

            {#if viewMode === "week"}
              <div class="py-2 text-center text-xs text-zinc-500 dark:text-zinc-400">
                {#if loadingMore}正在加载更多...{:else if hasMore}滚动加载更多{:else}没有更多了{/if}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    </div>

    <div class="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div
        class="rounded-3xl border border-white/20 bg-white/70 p-4 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="mb-3 text-sm font-medium text-zinc-900 dark:text-zinc-50">专注时段分析</div>
        <FocusAnalysisView {analysis} loading={analysisLoading} error={analysisError} />
      </div>
      <div
        class="rounded-3xl border border-white/20 bg-white/70 p-4 shadow-xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="mb-3 text-sm font-medium text-zinc-900 dark:text-zinc-50">中断分析</div>
        <InterruptionStatsView stats={interruptionStats} loading={interruptionLoading} error={interruptionError} />
      </div>
    </div>
  </div>
</main>

<ExportModal open={exportOpen} defaultRange={range} on:close={closeExport} on:submit={onExportSubmit} />
