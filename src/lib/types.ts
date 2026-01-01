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
}

/** 黑名单条目。 */
export interface BlacklistItem {
  /** 进程名（例如 `WeChat.exe`）。 */
  name: string;
  /** 展示名（例如 `微信`）。 */
  displayName: string;
}

/** 单条历史记录。 */
export interface HistoryRecord {
  /** 任务标签。 */
  tag: string;
  /** 开始时间（HH:mm）。 */
  startTime: string;
  /** 番茄时长（分钟）。 */
  duration: number;
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
  /** 标签历史。 */
  tags: string[];
  /** 历史记录。 */
  history: HistoryDay[];
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
