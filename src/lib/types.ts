/** 后端（Rust）与前端（Svelte）共享的数据类型定义（与 PRD 数据结构对齐）。 */

/** 计时器阶段（与 Rust `Phase` 的 `camelCase` 序列化一致）。 */
export type Phase = "work" | "shortBreak" | "longBreak";

/** 番茄钟设置。 */
export interface Settings {
  /** 工作时长（分钟）。 */
  pomodoro: number;
  /** 短休息时长（分钟）。 */
  shortBreak: number;
  /** 长休息时长（分钟）。 */
  longBreak: number;
  /** 长休息间隔（每 N 个番茄触发）。 */
  longBreakInterval: number;
  /** 是否启用“休息结束后自动进入工作倒计时”（连续番茄模式）。 */
  autoContinueEnabled: boolean;
  /** 连续番茄数量：在该次数内，休息结束后自动开始下一次工作倒计时。 */
  autoContinuePomodoros: number;
  /** 每日目标番茄数量（0 表示不设目标）。 */
  dailyGoal: number;
  /** 每周目标番茄数量（0 表示不设目标）。 */
  weeklyGoal: number;
  /** 是否置顶主窗口。 */
  alwaysOnTop: boolean;
}

/** 黑名单条目。 */
export interface BlacklistItem {
  /** 进程名（例如 `WeChat.exe`）。 */
  name: string;
  /** 展示名（例如 `微信`）。 */
  displayName: string;
}

/** 黑名单模板（内置/自定义）。 */
export interface BlacklistTemplate {
  /** 模板 id。 */
  id: string;
  /** 模板名称。 */
  name: string;
  /** 是否内置（内置模板不可删除）。 */
  builtin: boolean;
  /** 模板进程列表。 */
  processes: BlacklistItem[];
}

/** 单条历史记录。 */
export interface HistoryRecord {
  /** 任务标签。 */
  tag: string;
  /** 开始时间（HH:mm）。 */
  startTime: string;
  /** 结束时间（HH:mm；旧数据可能缺失）。 */
  endTime?: string | null;
  /** 番茄时长（分钟）。 */
  duration: number;
  /** 阶段类型（用于导出/分析）。 */
  phase: Phase;
  /** 备注（完成后填写，也可在历史中编辑）。 */
  remark: string;
}

/** 某一天的历史集合。 */
export interface HistoryDay {
  /** 日期（YYYY-MM-DD）。 */
  date: string;
  /** 记录列表。 */
  records: HistoryRecord[];
}

/** 持久化数据根对象。 */
export interface AppData {
  /** 用户设置。 */
  settings: Settings;
  /** 进程黑名单。 */
  blacklist: BlacklistItem[];
  /** 黑名单模板列表。 */
  blacklistTemplates: BlacklistTemplate[];
  /** 当前启用的模板 id 列表（可同时启用多套）。 */
  activeTemplateIds: string[];
  /** 兼容字段：当前单一模板 id（若存在）。 */
  activeTemplateId?: string | null;
  /** 标签历史。 */
  tags: string[];
  /** 历史记录。 */
  history: HistoryDay[];
  /** 调试历史记录（仅开发环境使用，与正式数据隔离）。 */
  historyDev: HistoryDay[];
}

/** 标签计数条目。 */
export interface TagCount {
  /** 标签名。 */
  tag: string;
  /** 完成次数。 */
  count: number;
}

/** 今日统计。 */
export interface TodayStats {
  /** 今日完成番茄总数。 */
  total: number;
  /** 按标签分组统计。 */
  byTag: TagCount[];
}

/** 本周统计。 */
export interface WeekStats {
  /** 本周完成番茄总数。 */
  total: number;
  /** 按标签分组统计。 */
  byTag: TagCount[];
}

/** 目标进度（每日/每周）。 */
export interface GoalProgress {
  /** 每日目标（0 表示不设目标）。 */
  dailyGoal: number;
  /** 今日已完成。 */
  dailyCompleted: number;
  /** 每周目标（0 表示不设目标）。 */
  weeklyGoal: number;
  /** 本周已完成。 */
  weeklyCompleted: number;
}

/** 计时器快照（用于 UI 渲染与托盘更新）。 */
export interface TimerSnapshot {
  /** 当前阶段。 */
  phase: Phase;
  /** 剩余秒数。 */
  remainingSeconds: number;
  /** 是否运行中。 */
  isRunning: boolean;
  /** 当前任务标签。 */
  currentTag: string;
  /** 专注期内是否锁定黑名单（只能增不能减）。 */
  blacklistLocked: boolean;
  /** 当前设置。 */
  settings: Settings;
  /** 今日统计。 */
  todayStats: TodayStats;
  /** 本周统计。 */
  weekStats: WeekStats;
  /** 目标进度。 */
  goalProgress: GoalProgress;
}

/** 前端首屏加载所需的完整快照。 */
export interface AppSnapshot {
  /** 持久化数据。 */
  data: AppData;
  /** 计时器状态。 */
  timer: TimerSnapshot;
}

/** 应用持久化 store 的真实存储路径（目录 + 文件）。 */
export interface StorePaths {
  /** Store 所在目录路径（可用于打开文件夹）。 */
  storeDirPath: string;
}

/** 运行进程信息。 */
export interface ProcessInfo {
  /** 进程名。 */
  name: string;
  /** 代表性 PID。 */
  pid: number;
  /** 可执行文件路径。 */
  exePath?: string | null;
  /** 图标 data URL。 */
  iconDataUrl?: string | null;
}

/** 单个进程名的终止结果。 */
export interface KillItem {
  /** 进程名。 */
  name: string;
  /** 尝试终止的 PID 列表。 */
  pids: number[];
  /** 成功数量。 */
  killed: number;
  /** 失败数量。 */
  failed: number;
  /** 是否需要管理员权限。 */
  requiresAdmin: boolean;
}

/** 一次批量终止的汇总结果。 */
export interface KillSummary {
  /** 结果明细。 */
  items: KillItem[];
  /** 是否有任何条目需要管理员权限。 */
  requiresAdmin: boolean;
}

/** 日期范围（闭区间）。 */
export interface DateRange {
  /** 起始日期（YYYY-MM-DD）。 */
  from: string;
  /** 结束日期（YYYY-MM-DD）。 */
  to: string;
}

/** 专注分析：标签效率条目。 */
export interface TagEfficiency {
  /** 标签名。 */
  tag: string;
  /** 平均时长（分钟）。 */
  avgDuration: number;
  /** 样本数（番茄数量）。 */
  count: number;
}

/** 专注时段分析结果。 */
export interface FocusAnalysis {
  /** 24 小时分布。 */
  hourlyCounts: number[];
  /** 时段分布：`[0-6,6-12,12-18,18-24]`。 */
  periodCounts: number[];
  /** 星期分布：`[周一..周日]`。 */
  weekdayCounts: number[];
  /** 交叉热力：`weekdayHourCounts[weekday][hour]`（7x24）。 */
  weekdayHourCounts: number[][];
  /** 标签效率。 */
  tagEfficiency: TagEfficiency[];
  /** 文字总结。 */
  summary: string;
}

/** 工作阶段完成事件（用于弹出备注填写）。 */
export interface WorkCompletedEvent {
  /** 日期（YYYY-MM-DD）。 */
  date: string;
  /** 当日记录索引。 */
  recordIndex: number;
  /** 新写入记录。 */
  record: HistoryRecord;
}

/** 导出格式。 */
export type ExportFormat = "csv" | "json";

/** 导出字段。 */
export type ExportField = "date" | "startTime" | "endTime" | "duration" | "tag" | "phase" | "remark";

/** 导出请求参数。 */
export interface ExportRequest {
  /** 导出范围。 */
  range: DateRange;
  /** 导出格式。 */
  format: ExportFormat;
  /** 导出字段（为空则使用默认字段集）。 */
  fields: ExportField[];
}
