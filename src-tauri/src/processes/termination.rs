//! 进程终止与权限检测（用于专注模式自动清理干扰程序）。

use serde::{Deserialize, Serialize};
use sysinfo::System;
use ts_rs::TS;

#[cfg(windows)]
use crate::errors::AppError;
use crate::errors::AppResult;

/// 单个进程名的终止结果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct KillItem {
    /// 进程名。
    pub name: String,
    /// 尝试终止的 PID 列表。
    pub pids: Vec<u32>,
    /// 成功数量。
    pub killed: u32,
    /// 失败数量。
    pub failed: u32,
    /// 是否存在“需要管理员权限”导致的失败。
    pub requires_admin: bool,
}

/// 一次批量终止的汇总结果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct KillSummary {
    /// 各进程名的明细。
    pub items: Vec<KillItem>,
    /// 是否有任何条目需要管理员权限。
    pub requires_admin: bool,
}

/// 终止所有匹配 `process_name` 的进程（返回可用于 UI 展示的汇总结果）。
fn kill_by_name(process_name: &str) -> AppResult<KillSummary> {
    tracing::debug!(target: "blacklist", "尝试终止进程：{}", process_name);
    let mut system = System::new_all();
    system.refresh_all();

    let mut items: Vec<KillItem> = Vec::new();
    let mut any_requires_admin = false;

    let mut pids: Vec<u32> = system
        .processes()
        .iter()
        .filter_map(|(pid, p)| {
            if eq_process_name(p.name(), process_name) {
                Some(pid.as_u32())
            } else {
                None
            }
        })
        .collect();

    pids.sort_unstable();

    if pids.is_empty() {
        tracing::debug!(target: "blacklist", "未找到匹配进程：{}", process_name);
        return Ok(KillSummary {
            items: vec![KillItem {
                name: process_name.to_string(),
                pids,
                killed: 0,
                failed: 0,
                requires_admin: false,
            }],
            requires_admin: false,
        });
    }

    let mut killed = 0u32;
    let mut failed = 0u32;
    #[cfg(windows)]
    let mut requires_admin = false;
    #[cfg(not(windows))]
    let requires_admin = false;

    for pid in &pids {
        match kill_pid(*pid) {
            Ok(true) => killed += 1,
            Ok(false) => {
                failed += 1;
            }
            #[cfg(windows)]
            Err(AppError::KillFailed(msg)) => {
                failed += 1;
                if msg.contains("ACCESS_DENIED") || msg.contains("权限") {
                    requires_admin = true;
                }
            }
            Err(e) => return Err(e),
        }
    }

    any_requires_admin |= requires_admin;
    items.push(KillItem {
        name: process_name.to_string(),
        pids,
        killed,
        failed,
        requires_admin,
    });

    if failed > 0 {
        tracing::warn!(
            target: "blacklist",
            "终止进程存在失败：name={} killed={} failed={} requiresAdmin={}",
            process_name,
            killed,
            failed,
            requires_admin
        );
    } else {
        tracing::info!(
            target: "blacklist",
            "终止进程成功：name={} killed={}",
            process_name,
            killed
        );
    }

    Ok(KillSummary {
        items,
        requires_admin: any_requires_admin,
    })
}

/// 批量终止若干进程名（best-effort）：忽略单个名称的错误并合并为一次汇总结果。
pub fn kill_names_best_effort(names: &[String]) -> KillSummary {
    if names.is_empty() {
        return KillSummary {
            items: Vec::new(),
            requires_admin: false,
        };
    }

    let mut all_items = Vec::new();
    let mut requires_admin = false;

    for name in names {
        if let Ok(summary) = kill_by_name(name) {
            requires_admin |= summary.requires_admin;
            all_items.extend(summary.items);
        }
    }

    KillSummary {
        items: all_items,
        requires_admin,
    }
}

/// 以 PID 终止进程（返回是否成功）。
fn kill_pid(pid: u32) -> AppResult<bool> {
    #[cfg(windows)]
    {
        kill_pid_windows(pid)
    }

    #[cfg(not(windows))]
    {
        kill_pid_fallback(pid)
    }
}

/// 非 Windows 平台的兜底终止实现（用于开发环境）。
#[cfg(not(windows))]
fn kill_pid_fallback(pid: u32) -> AppResult<bool> {
    let mut system = System::new_all();
    system.refresh_all();
    let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
        return Ok(false);
    };
    Ok(process.kill())
}

/// Windows 平台：通过 Win32 API 强制终止指定 PID。
#[cfg(windows)]
fn kill_pid_windows(pid: u32) -> AppResult<bool> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    /// 进程句柄守卫：确保 `CloseHandle` 被调用。
    struct HandleGuard(windows::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        /// 释放进程句柄。
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
            AppError::KillFailed(format!(
                "OpenProcess 失败（{e:?}）ACCESS_DENIED 可能需要管理员权限"
            ))
        })?;
        let handle = HandleGuard(handle);

        let result = TerminateProcess(handle.0, 1).map(|_| true).map_err(|e| {
            AppError::KillFailed(format!(
                "TerminateProcess 失败（{e:?}）ACCESS_DENIED 可能需要管理员权限"
            ))
        });

        result
    }
}

/// 进程名对比（Windows 下不区分大小写）。
fn eq_process_name(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}
