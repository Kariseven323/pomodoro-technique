/** 前端诊断工具：捕获未处理异常并写入后端日志（用于定位“白屏/卡加载”等问题）。 */

import { isTauri } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import { frontendLog } from "$lib/api/tauri";
import { appError, appLoading } from "$lib/stores/appClient";

let installed = false;
let fatalUiHandled = false;
let lastErrorSignature: string | null = null;
let lastErrorAt = 0;

/** 判断当前是否运行在 Tauri 环境（避免浏览器环境调用 invoke 导致 pending）。 */
function isTauriRuntime(): boolean {
  try {
    return isTauri();
  } catch {
    return false;
  }
}

/** 将未知错误对象转为尽量可读的字符串（包含 stack/JSON 的 best-effort）。 */
function formatUnknownError(reason: unknown): string {
  if (reason instanceof Error) {
    const stack = reason.stack?.trim();
    return stack ? `${reason.message}\n${stack}` : reason.message;
  }
  if (typeof reason === "string") return reason;
  try {
    return JSON.stringify(reason);
  } catch {
    return String(reason);
  }
}

/** 将错误信息写入后端日志（release 仅落盘 warn/error；失败时静默）。 */
async function logToBackend(level: "warn" | "error", message: string): Promise<void> {
  if (!isTauriRuntime()) return;
  try {
    await frontendLog(level, message);
  } catch {
    // 忽略：日志仅用于诊断，不应影响主流程
  }
}

/** 判断某条错误是否应当写入后端日志（去重 + 简单节流，避免无限报错写爆日志）。 */
function shouldLogError(message: string): boolean {
  const now = Date.now();
  const signature = message.slice(0, 600);
  const sameAsLast = lastErrorSignature === signature;
  const withinCooldown = now - lastErrorAt < 1500;
  if (sameAsLast && withinCooldown) return false;
  lastErrorSignature = signature;
  lastErrorAt = now;
  return true;
}

/** 在捕获到“疑似致命”的前端异常时，尽量把 UI 从 loading 中解救出来并展示错误（仅限初始化阶段）。 */
function markFatalAndStopLoading(message: string): void {
  if (fatalUiHandled) return;
  if (!get(appLoading)) return;
  fatalUiHandled = true;
  appError.set(message);
  appLoading.set(false);
}

/** 安装全局错误/未处理 Promise 拒绝监听（全局只安装一次）。 */
export function installFrontendErrorLogging(): void {
  if (installed) return;
  installed = true;

  window.addEventListener("error", (ev) => {
    const parts = [
      "[frontend] window.error",
      ev.message ? `message=${ev.message}` : "",
      ev.filename ? `file=${ev.filename}:${ev.lineno}:${ev.colno}` : "",
    ].filter(Boolean);
    const errText = ev.error ? `\n${formatUnknownError(ev.error)}` : "";
    const msg = `${parts.join(" ")}${errText}`;
    if (shouldLogError(msg)) void logToBackend("error", msg);
    markFatalAndStopLoading("前端发生未处理错误，已停止加载；请查看日志目录定位原因。");
  });

  window.addEventListener("unhandledrejection", (ev) => {
    const msg = `[frontend] unhandledrejection reason=${formatUnknownError(ev.reason)}`;
    if (shouldLogError(msg)) void logToBackend("error", msg);
    markFatalAndStopLoading("前端发生未处理 Promise 拒绝，已停止加载；请查看日志目录定位原因。");
  });
}
