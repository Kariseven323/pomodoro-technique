/** Tauri 后端命令调用封装（集中处理 invoke、类型与命令名）。 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppSnapshot,
  BlacklistItem,
  BlacklistTemplate,
  DateRange,
  ExportRequest,
  FocusAnalysis,
  HistoryDay,
  HistoryRecord,
  ProcessInfo,
  Settings,
  StorePaths,
  TimerSnapshot,
} from "../shared/types";

/** 获取应用完整快照（持久化数据 + 计时器状态）。 */
export async function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

/** 获取应用数据根目录路径（统一入口）。 */
export async function getStorePaths(): Promise<StorePaths> {
  return invoke<StorePaths>("get_store_paths");
}

/** 打开应用数据根目录（统一入口，便于一处查看与备份）。 */
export async function openStoreDir(): Promise<void> {
  return invoke<void>("open_store_dir");
}

/** 更新设置（后端会进行范围校验并持久化）。 */
export async function updateSettings(settings: Settings): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("update_settings", { settings });
}

/** 设置每日/每周目标（0 表示不设目标）。 */
export async function setGoals(daily: number, weekly: number): Promise<Settings> {
  return invoke<Settings>("set_goals", { daily, weekly });
}

/** 设置当前番茄的任务标签（若为新标签会自动加入标签历史）。 */
export async function setCurrentTag(tag: string): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("set_current_tag", { tag });
}

/** 新增标签到历史列表（不会自动选中）。 */
export async function addTag(tag: string): Promise<string[]> {
  return invoke<string[]>("add_tag", { tag });
}

/** 设置黑名单（专注期内仅允许新增，不允许移除）。 */
export async function setBlacklist(blacklist: BlacklistItem[]): Promise<BlacklistItem[]> {
  return invoke<BlacklistItem[]>("set_blacklist", { blacklist });
}

/** 获取指定范围的历史记录（按日分组）。 */
export async function getHistory(range: DateRange): Promise<HistoryDay[]> {
  return invoke<HistoryDay[]>("get_history", { range });
}

/** 设置某条历史记录的备注。 */
export async function setHistoryRemark(date: string, recordIndex: number, remark: string): Promise<HistoryRecord> {
  return invoke<HistoryRecord>("set_history_remark", { date, recordIndex, remark });
}

/** 获取指定范围的专注时段分析数据。 */
export async function getFocusAnalysis(range: DateRange): Promise<FocusAnalysis> {
  return invoke<FocusAnalysis>("get_focus_analysis", { range });
}

/** 获取全部黑名单模板。 */
export async function getTemplates(): Promise<BlacklistTemplate[]> {
  return invoke<BlacklistTemplate[]>("get_templates");
}

/** 保存黑名单模板（新增/更新自定义模板）。 */
export async function saveTemplate(template: BlacklistTemplate): Promise<BlacklistTemplate> {
  return invoke<BlacklistTemplate>("save_template", { template });
}

/** 删除自定义模板（内置模板不可删除）。 */
export async function deleteTemplate(id: string): Promise<boolean> {
  return invoke<boolean>("delete_template", { id });
}

/** 应用/切换模板（支持同时启用多套），返回应用后的黑名单。 */
export async function applyTemplate(id: string): Promise<BlacklistItem[]> {
  return invoke<BlacklistItem[]>("apply_template", { id });
}

/** 设置主窗口置顶状态。 */
export async function setAlwaysOnTop(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_always_on_top", { enabled });
}

/** 切换迷你模式（窗口调整为 200x80，仅显示倒计时）。 */
export async function setMiniMode(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_mini_mode", { enabled });
}

/** 导出历史记录：弹出保存对话框并写入 CSV/JSON，返回保存路径。 */
export async function exportHistory(request: ExportRequest): Promise<string> {
  return invoke<string>("export_history", { request });
}

/** 打开日志目录（文件管理器）。 */
export async function openLogDir(): Promise<boolean> {
  return invoke<boolean>("open_log_dir");
}

/** 前端诊断日志：写入后端 tracing 文件日志（用于定位 WebView/布局问题）。 */
export async function frontendLog(level: "debug" | "info" | "warn" | "error", message: string): Promise<boolean> {
  return invoke<boolean>("frontend_log", { level, message });
}

/** 退出应用（用于迷你模式右键菜单）。 */
export async function exitApp(): Promise<boolean> {
  return invoke<boolean>("exit_app");
}

/** 开发者命令：生成测试历史数据（仅开发环境可用）。 */
export async function debugGenerateHistory(days: number): Promise<number> {
  return invoke<number>("debug_generate_history", { days });
}

/** 开发者命令：清空测试历史数据（仅开发环境可用）。 */
export async function debugClearHistory(): Promise<boolean> {
  return invoke<boolean>("debug_clear_history");
}

/** 获取当前运行进程列表（进程名 + 图标）。 */
export async function listProcesses(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>("list_processes");
}

/** 开始计时（工作阶段首次开始会自动终止黑名单进程）。 */
export async function timerStart(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>("timer_start");
}

/** 暂停计时。 */
export async function timerPause(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>("timer_pause");
}

/** 重置计时（回到工作阶段初始状态）。 */
export async function timerReset(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>("timer_reset");
}

/** 跳过当前阶段（不会写入历史）。 */
export async function timerSkip(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>("timer_skip");
}

/** Windows：以管理员身份重启应用（用于终止需要提权的进程）。 */
export async function restartAsAdmin(): Promise<void> {
  return invoke<void>("restart_as_admin");
}
