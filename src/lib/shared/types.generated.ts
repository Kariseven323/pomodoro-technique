/**
 * 本文件由 `cargo run --bin gen_ts_types` 自动生成，请勿手动编辑。
 *
 * 生成来源：Rust 端 `ts-rs` derive（序列化字段名与前端保持 camelCase 一致）。
 */

export type Phase = "work" | "shortBreak" | "longBreak";
export type Settings = {
  /**
   * 工作时长（分钟）。
   */
  pomodoro: number;
  /**
   * 短休息时长（分钟）。
   */
  shortBreak: number;
  /**
   * 长休息时长（分钟）。
   */
  longBreak: number;
  /**
   * 长休息间隔（每 N 个番茄触发）。
   */
  longBreakInterval: number;
  /**
   * 是否启用“休息结束后自动进入工作倒计时”（连续番茄模式）。
   */
  autoContinueEnabled: boolean;
  /**
   * 连续番茄数量：在该次数内，休息结束后自动开始下一次工作倒计时。
   */
  autoContinuePomodoros: number;
  /**
   * 每日目标番茄数量（0 表示不设目标）。
   */
  dailyGoal: number;
  /**
   * 每周目标番茄数量（0 表示不设目标）。
   */
  weeklyGoal: number;
  /**
   * 窗口是否置顶（主窗口）。
   */
  alwaysOnTop: boolean;
};
export type BlacklistItem = {
  /**
   * 进程名（例如 `WeChat.exe`）。
   */
  name: string;
  /**
   * 展示名（例如 `微信`）。
   */
  displayName: string;
};
export type BlacklistTemplate = {
  /**
   * 模板 id（内置模板为固定值，自定义模板可为 uuid/自定义字符串）。
   */
  id: string;
  /**
   * 模板名称。
   */
  name: string;
  /**
   * 是否为内置模板（内置模板不可删除）。
   */
  builtin: boolean;
  /**
   * 模板包含的黑名单进程集合。
   */
  processes: Array<BlacklistItem>;
};
export type HistoryRecord = {
  /**
   * 任务标签。
   */
  tag: string;
  /**
   * 开始时间（HH:mm）。
   */
  startTime: string;
  /**
   * 结束时间（HH:mm；旧数据可能缺失，前端可按 `start_time + duration` 推导展示）。
   */
  endTime: string | null;
  /**
   * 本次番茄时长（分钟）。
   */
  duration: number;
  /**
   * 阶段类型（用于导出/分析；当前仅记录工作阶段）。
   */
  phase: Phase;
  /**
   * 备注（完成后可填写，也可在历史中编辑）。
   */
  remark: string;
};
export type HistoryDay = {
  /**
   * 日期（YYYY-MM-DD）。
   */
  date: string;
  /**
   * 当日记录。
   */
  records: Array<HistoryRecord>;
};
export type AppData = {
  /**
   * 用户设置。
   */
  settings: Settings;
  /**
   * 进程黑名单。
   */
  blacklist: Array<BlacklistItem>;
  /**
   * 黑名单模板列表（包含内置模板与自定义模板）。
   */
  blacklistTemplates: Array<BlacklistTemplate>;
  /**
   * 当前启用的模板 id 列表（支持同时启用多套模板）。
   */
  activeTemplateIds: Array<string>;
  /**
   * 兼容字段：旧/示例数据中的单一激活模板（用于自动迁移到 `active_template_ids`）。
   */
  activeTemplateId: string | null;
  /**
   * 历史标签。
   */
  tags: Array<string>;
  /**
   * 历史记录（按日分组）。
   */
  history: Array<HistoryDay>;
  /**
   * 调试历史记录（仅开发环境使用，与正式数据隔离）。
   */
  historyDev: Array<HistoryDay>;
};
export type TagCount = {
  /**
   * 标签名。
   */
  tag: string;
  /**
   * 完成次数。
   */
  count: number;
};
export type TodayStats = {
  /**
   * 今日完成的番茄总数。
   */
  total: number;
  /**
   * 按标签统计。
   */
  byTag: Array<TagCount>;
};
export type WeekStats = {
  /**
   * 本周完成的番茄总数。
   */
  total: number;
  /**
   * 按标签统计。
   */
  byTag: Array<TagCount>;
};
export type GoalProgress = {
  /**
   * 每日目标（0 表示未设置）。
   */
  dailyGoal: number;
  /**
   * 今日已完成。
   */
  dailyCompleted: number;
  /**
   * 每周目标（0 表示未设置）。
   */
  weeklyGoal: number;
  /**
   * 本周已完成。
   */
  weeklyCompleted: number;
};
export type TimerSnapshot = {
  /**
   * 当前阶段。
   */
  phase: Phase;
  /**
   * 剩余秒数。
   */
  remainingSeconds: bigint;
  /**
   * 是否运行中。
   */
  isRunning: boolean;
  /**
   * 当前任务标签。
   */
  currentTag: string;
  /**
   * 专注期内黑名单是否锁定（只能增不能减）。
   */
  blacklistLocked: boolean;
  /**
   * 当前设置（用于前端展示/校验）。
   */
  settings: Settings;
  /**
   * 今日统计（用于主界面展示）。
   */
  todayStats: TodayStats;
  /**
   * 本周统计（用于主界面展示）。
   */
  weekStats: WeekStats;
  /**
   * 目标进度（用于主界面展示与提醒判断）。
   */
  goalProgress: GoalProgress;
};
export type WorkCompletedEvent = {
  /**
   * 记录日期（YYYY-MM-DD）。
   */
  date: string;
  /**
   * 当日记录索引（从 0 开始）。
   */
  recordIndex: number;
  /**
   * 写入的记录内容。
   */
  record: HistoryRecord;
};
export type AppSnapshot = {
  /**
   * 持久化数据（settings/blacklist/tags/history）。
   */
  data: AppData;
  /**
   * 计时器状态快照。
   */
  timer: TimerSnapshot;
};
export type StorePaths = {
  /**
   * 数据根目录路径（统一入口，可用于打开文件夹）。
   */
  storeDirPath: string;
};
export type ProcessInfo = {
  /**
   * 进程名（例如 `WeChat.exe`）。
   */
  name: string;
  /**
   * 代表性 PID（用于展示）。
   */
  pid: number;
  /**
   * 可执行文件路径（若可获取）。
   */
  exePath: string | null;
  /**
   * 进程图标（data URL：`data:image/png;base64,...`）。
   */
  iconDataUrl: string | null;
};
export type KillItem = {
  /**
   * 进程名。
   */
  name: string;
  /**
   * 尝试终止的 PID 列表。
   */
  pids: Array<number>;
  /**
   * 成功数量。
   */
  killed: number;
  /**
   * 失败数量。
   */
  failed: number;
  /**
   * 是否存在“需要管理员权限”导致的失败。
   */
  requiresAdmin: boolean;
};
export type KillSummary = {
  /**
   * 各进程名的明细。
   */
  items: Array<KillItem>;
  /**
   * 是否有任何条目需要管理员权限。
   */
  requiresAdmin: boolean;
};
export type DateRange = {
  /**
   * 起始日期（YYYY-MM-DD）。
   */
  from: string;
  /**
   * 结束日期（YYYY-MM-DD）。
   */
  to: string;
};
export type TagEfficiency = {
  /**
   * 标签名。
   */
  tag: string;
  /**
   * 平均时长（分钟）。
   */
  avgDuration: number;
  /**
   * 样本数（番茄数量）。
   */
  count: number;
};
export type FocusAnalysis = {
  /**
   * 24 小时分布（按 `startTime` 的小时计数）。
   */
  hourlyCounts: Array<number>;
  /**
   * 时段分布：`[0-6, 6-12, 12-18, 18-24]`。
   */
  periodCounts: Array<number>;
  /**
   * 星期分布：`[周一..周日]`。
   */
  weekdayCounts: Array<number>;
  /**
   * 交叉热力：`weekday_hour_counts[weekday][hour]`（7x24）。
   */
  weekdayHourCounts: Array<Array<number>>;
  /**
   * 标签效率：各标签平均专注时长（分钟）。
   */
  tagEfficiency: Array<TagEfficiency>;
  /**
   * 文字总结（示例：「你在上午 9-11 点专注效率最高」）。
   */
  summary: string;
};
export type ExportFormat = "csv" | "json";
export type ExportField = "date" | "startTime" | "endTime" | "duration" | "tag" | "phase" | "remark";
export type ExportRequest = {
  /**
   * 导出范围。
   */
  range: DateRange;
  /**
   * 导出格式。
   */
  format: ExportFormat;
  /**
   * 导出字段（为空则导出默认字段集）。
   */
  fields: Array<ExportField>;
};
