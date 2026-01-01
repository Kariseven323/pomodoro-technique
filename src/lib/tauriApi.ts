/** Tauri 后端命令调用封装（集中处理 invoke、类型与命令名）。 */

import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, BlacklistItem, ProcessInfo, Settings, TimerSnapshot } from "./types";

/** 获取应用完整快照（持久化数据 + 计时器状态）。 */
export async function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

/** 更新设置（后端会进行范围校验并持久化）。 */
export async function updateSettings(settings: Settings): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("update_settings", { settings });
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

