//! 进程相关 IPC 命令：列举进程、提权重启等。

use crate::commands::common::to_ipc_result;
use crate::commands::processes::restart_as_admin_impl;
use crate::errors::{AppError, AppResult};
use crate::processes::{self, ProcessInfo};

/// 获取当前运行的进程列表（进程名 + 图标）。
#[tauri::command]
pub async fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    to_ipc_result(list_processes_impl().await)
}

/// `list_processes` 的内部实现：放到阻塞线程池执行，避免卡住主线程事件循环。
async fn list_processes_impl() -> AppResult<Vec<ProcessInfo>> {
    let started = std::time::Instant::now();
    tracing::warn!(target: "processes", "list_processes 开始");

    let out = tauri::async_runtime::spawn_blocking(processes::list_processes)
        .await
        .map_err(|e| AppError::Invariant(format!("进程枚举任务 join 失败：{e}")))??;

    let exe_count = out
        .iter()
        .filter(|p| p.exe_path.as_deref().unwrap_or("").trim().len() > 0)
        .count();
    tracing::warn!(
        target: "processes",
        "list_processes 完成：count={} exe_count={} cost_ms={}",
        out.len(),
        exe_count,
        started.elapsed().as_millis()
    );
    Ok(out)
}

/// 获取某个 exe 的图标 data URL（用于黑名单管理按需加载）。
#[tauri::command]
pub async fn process_icon(exe_path: String) -> Result<Option<String>, String> {
    to_ipc_result(process_icon_impl(exe_path).await)
}

/// `process_icon` 的内部实现：放到阻塞线程池执行，避免卡住主线程事件循环。
async fn process_icon_impl(exe_path: String) -> AppResult<Option<String>> {
    let exe_path = exe_path.trim().to_string();
    if exe_path.is_empty() {
        return Ok(None);
    }
    let started = std::time::Instant::now();
    tracing::warn!(target: "processes", "process_icon 开始：exe_path={}", exe_path);

    let out =
        tauri::async_runtime::spawn_blocking(move || processes::icon_data_url_for_exe(&exe_path))
            .await
            .map_err(|e| AppError::Invariant(format!("图标提取任务 join 失败：{e}")))??;

    tracing::warn!(
        target: "processes",
        "process_icon 完成：has_icon={} cost_ms={}",
        out.is_some(),
        started.elapsed().as_millis()
    );
    Ok(out)
}

/// Windows：以管理员身份重启（用于终止需要提权的进程）。
#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    to_ipc_result(restart_as_admin_impl())
}
