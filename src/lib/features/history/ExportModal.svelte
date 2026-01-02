<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { DateRange, ExportField, ExportFormat } from "$lib/shared/types";

  const props = $props<{ open: boolean; defaultRange: DateRange }>();

  const dispatch = createEventDispatcher<{
    close: void;
    submit: { range: DateRange; format: ExportFormat; fields: ExportField[] };
  }>();

  let wasOpen = $state(false);
  let range = $state<DateRange>({ from: "", to: "" });
  let format = $state<ExportFormat>("csv");
  let fields = $state<ExportField[]>(["date", "startTime", "endTime", "duration", "tag", "phase"]);

  /** 响应 open 变化：打开时同步默认范围。 */
  function onOpenEffect(): void {
    if (props.open && !wasOpen) {
      range = { ...props.defaultRange };
      format = "csv";
      fields = ["date", "startTime", "endTime", "duration", "tag", "phase"];
    }
    wasOpen = props.open;
  }

  $effect(onOpenEffect);

  /** 关闭弹窗（不导出）。 */
  function closeModal(): void {
    dispatch("close");
  }

  /** 切换字段勾选。 */
  function toggleField(field: ExportField, checked: boolean): void {
    if (checked) {
      if (!fields.includes(field)) fields = [...fields, field];
    } else {
      fields = fields.filter((x) => x !== field);
    }
  }

  /** 提交导出请求。 */
  function submit(): void {
    dispatch("submit", { range: { ...range }, format, fields: [...fields] });
  }
</script>

{#if props.open}
  <div class="fixed inset-0 z-50">
    <button
      type="button"
      class="absolute inset-0 bg-black/30 backdrop-blur-sm"
      aria-label="关闭弹窗"
      onclick={closeModal}
    ></button>
    <div class="absolute inset-0 flex items-center justify-center p-4">
      <div
        class="w-full max-w-lg rounded-3xl border border-white/20 bg-white/80 p-5 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/70"
      >
        <div class="mb-4 flex items-center justify-between gap-3">
          <div>
            <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-50">导出历史记录</h2>
            <p class="mt-1 text-xs text-zinc-600 dark:text-zinc-300">选择时间范围、格式与字段</p>
          </div>
          <button
            class="rounded-xl px-3 py-1 text-sm text-zinc-600 hover:bg-black/5 dark:text-zinc-300 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            关闭
          </button>
        </div>

        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">起始日期</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="date"
              bind:value={range.from}
            />
          </label>
          <label class="block">
            <div class="mb-1 text-sm text-zinc-700 dark:text-zinc-200">结束日期</div>
            <input
              class="w-full rounded-2xl border border-black/10 bg-white/70 px-3 py-2 text-sm text-zinc-900 outline-none dark:border-white/10 dark:bg-white/5 dark:text-zinc-50"
              type="date"
              bind:value={range.to}
            />
          </label>
        </div>

        <div class="mt-3 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
          <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">格式</div>
          <div class="flex items-center gap-2">
            <label class="flex items-center gap-2 text-sm">
              <input class="h-4 w-4" type="radio" name="fmt" checked={format === "csv"} onchange={() => (format = "csv")} />
              CSV
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input class="h-4 w-4" type="radio" name="fmt" checked={format === "json"} onchange={() => (format = "json")} />
              JSON
            </label>
          </div>
        </div>

        <div class="mt-3 rounded-2xl border border-black/10 bg-white/60 p-3 dark:border-white/10 dark:bg-white/5">
          <div class="mb-2 text-sm font-medium text-zinc-900 dark:text-zinc-50">字段</div>
          <div class="grid grid-cols-2 gap-2 text-sm sm:grid-cols-3">
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("date")} onchange={(e) => toggleField("date", (e.currentTarget as HTMLInputElement).checked)} />
              日期
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("startTime")} onchange={(e) => toggleField("startTime", (e.currentTarget as HTMLInputElement).checked)} />
              开始时间
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("endTime")} onchange={(e) => toggleField("endTime", (e.currentTarget as HTMLInputElement).checked)} />
              结束时间
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("duration")} onchange={(e) => toggleField("duration", (e.currentTarget as HTMLInputElement).checked)} />
              时长
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("tag")} onchange={(e) => toggleField("tag", (e.currentTarget as HTMLInputElement).checked)} />
              标签
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("phase")} onchange={(e) => toggleField("phase", (e.currentTarget as HTMLInputElement).checked)} />
              阶段
            </label>
            <label class="flex items-center gap-2">
              <input class="h-4 w-4" type="checkbox" checked={fields.includes("remark")} onchange={(e) => toggleField("remark", (e.currentTarget as HTMLInputElement).checked)} />
              备注
            </label>
          </div>
        </div>

        <div class="mt-5 flex items-center justify-end gap-2">
          <button
            class="rounded-2xl px-4 py-2 text-sm text-zinc-700 hover:bg-black/5 dark:text-zinc-200 dark:hover:bg-white/10"
            onclick={closeModal}
          >
            取消
          </button>
          <button
            class="rounded-2xl bg-zinc-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-zinc-800 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
            disabled={!range.from || !range.to}
            onclick={submit}
          >
            导出
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
