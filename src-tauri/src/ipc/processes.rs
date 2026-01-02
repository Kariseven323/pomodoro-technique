//! 进程相关 IPC 命令：列举进程、提权重启等。

use crate::commands::common::to_ipc_result;
use crate::commands::processes::restart_as_admin_impl;
use crate::processes::{self, ProcessInfo};

/// 获取当前运行的进程列表（进程名 + 图标）。
#[tauri::command]
pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    to_ipc_result(processes::list_processes())
}

/// Windows：以管理员身份重启（用于终止需要提权的进程）。
#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    to_ipc_result(restart_as_admin_impl())
}

