/** 计时器阶段相关的通用展示工具。 */

import type { Phase } from "$lib/shared/types";

/**
 * 将阶段映射为中文展示名。
 */
export function phaseLabel(phase: Phase): string {
  if (phase === "work") return "工作";
  if (phase === "shortBreak") return "短休息";
  return "长休息";
}

/**
 * 将阶段映射为强调色（Tailwind class）。
 */
export function phaseAccentClass(phase: Phase): string {
  if (phase === "work") return "text-rose-600 dark:text-rose-300";
  if (phase === "shortBreak") return "text-emerald-600 dark:text-emerald-300";
  return "text-sky-600 dark:text-sky-300";
}
