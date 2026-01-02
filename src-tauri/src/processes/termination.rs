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

    let entries = system
        .processes()
        .iter()
        .map(|(pid, p)| (pid.as_u32(), p.name().to_string()));
    kill_by_name_from_entries(process_name, entries, |pid| kill_pid(pid))
}

/// `kill_by_name` 的可测试实现：接受“进程快照条目”与可注入的 `kill_pid`，避免单测触发系统调用。
fn kill_by_name_from_entries<I>(
    process_name: &str,
    entries: I,
    mut kill_pid_fn: impl FnMut(u32) -> AppResult<bool>,
) -> AppResult<KillSummary>
where
    I: IntoIterator<Item = (u32, String)>,
{
    let mut pids: Vec<u32> = entries
        .into_iter()
        .filter_map(|(pid, name)| {
            if eq_process_name(&name, process_name) {
                Some(pid)
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

    let (killed, failed, requires_admin) = kill_pids(&pids, &mut kill_pid_fn)?;
    let items = vec![KillItem {
        name: process_name.to_string(),
        pids,
        killed,
        failed,
        requires_admin,
    }];

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
        requires_admin,
    })
}

/// 逐个终止 PID 列表，并返回 `(killed, failed, requires_admin)` 汇总。
fn kill_pids(
    pids: &[u32],
    kill_pid_fn: &mut impl FnMut(u32) -> AppResult<bool>,
) -> AppResult<(u32, u32, bool)> {
    let mut killed = 0u32;
    let mut failed = 0u32;
    #[cfg(windows)]
    let mut requires_admin = false;
    #[cfg(not(windows))]
    let requires_admin = false;

    for pid in pids {
        match kill_pid_fn(*pid) {
            Ok(true) => killed += 1,
            Ok(false) => failed += 1,
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

    Ok((killed, failed, requires_admin))
}

/// 批量终止若干进程名（best-effort）：忽略单个名称的错误并合并为一次汇总结果。
pub fn kill_names_best_effort(names: &[String]) -> KillSummary {
    kill_names_best_effort_with(names, |name| kill_by_name(name))
}

/// `kill_names_best_effort` 的可注入实现：便于在单元测试中 mock `kill_by_name`。
fn kill_names_best_effort_with(
    names: &[String],
    mut kill_by_name_fn: impl FnMut(&str) -> AppResult<KillSummary>,
) -> KillSummary {
    if names.is_empty() {
        return KillSummary {
            items: Vec::new(),
            requires_admin: false,
        };
    }

    let mut all_items = Vec::new();
    let mut requires_admin = false;

    for name in names {
        if let Ok(summary) = kill_by_name_fn(name) {
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
#[cfg(windows)]
fn eq_process_name(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// 进程名对比（非 Windows：保持大小写敏感，避免误杀）。
#[cfg(not(windows))]
fn eq_process_name(a: &str, b: &str) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Once;

    /// 初始化 `tracing`（仅一次）：确保日志字段参数会被求值，便于覆盖率统计。
    fn init_tracing_once() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .with_test_writer()
                .try_init();
        });
    }

    /// `kill_names_best_effort_with`：空列表应返回空结果。
    #[test]
    fn kill_names_best_effort_handles_empty_list() {
        let out = kill_names_best_effort_with(&[], |_name| unreachable!("不应被调用"));
        assert!(out.items.is_empty());
        assert!(!out.requires_admin);
    }

    /// `kill_names_best_effort_with`：应合并多个名称的汇总结果，并忽略单个名称的错误。
    #[test]
    fn kill_names_best_effort_merges_and_ignores_errors() {
        let names = vec![
            "a.exe".to_string(),
            "b.exe".to_string(),
            "bad.exe".to_string(),
        ];
        let out = kill_names_best_effort_with(&names, |name| {
            if name == "bad.exe" {
                return Err(crate::errors::AppError::Invariant("boom".to_string()));
            }
            Ok(KillSummary {
                items: vec![KillItem {
                    name: name.to_string(),
                    pids: vec![1],
                    killed: 1,
                    failed: 0,
                    requires_admin: name == "b.exe",
                }],
                requires_admin: name == "b.exe",
            })
        });

        assert_eq!(out.items.len(), 2);
        assert!(out.items.iter().any(|it| it.name == "a.exe"));
        assert!(out.items.iter().any(|it| it.name == "b.exe"));
        assert!(out.requires_admin);
    }

    /// `kill_names_best_effort`：空列表应直接返回空结果（且不会触发系统调用）。
    #[test]
    fn kill_names_best_effort_public_empty_list_is_safe() {
        let out = kill_names_best_effort(&[]);
        assert!(out.items.is_empty());
        assert!(!out.requires_admin);
    }

    /// `kill_by_name_from_entries`：无匹配进程时应返回空 PID 列表与 0 计数。
    #[test]
    fn kill_by_name_from_entries_handles_no_match() {
        let out = kill_by_name_from_entries("a.exe", vec![(1, "b.exe".to_string())], |_pid| {
            unreachable!("无匹配时不应调用 kill_pid")
        })
        .unwrap();
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].name, "a.exe");
        assert!(out.items[0].pids.is_empty());
        assert_eq!(out.items[0].killed, 0);
        assert_eq!(out.items[0].failed, 0);
    }

    /// `kill_by_name_from_entries`：应筛选并排序 PID，并正确累计 killed/failed。
    #[test]
    fn kill_by_name_from_entries_sorts_and_counts() {
        init_tracing_once();
        let entries = vec![
            (3, "a.exe".to_string()),
            (1, "a.exe".to_string()),
            (2, "a.exe".to_string()),
            (9, "b.exe".to_string()),
        ];
        let out = kill_by_name_from_entries("a.exe", entries, |pid| match pid {
            1 | 3 => Ok(true),
            2 => Ok(false),
            _ => Ok(false),
        })
        .unwrap();

        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].pids, vec![1, 2, 3]);
        assert_eq!(out.items[0].killed, 2);
        assert_eq!(out.items[0].failed, 1);
    }

    /// `kill_by_name_from_entries`：当全部终止成功时应走到“成功日志”分支（failed=0）。
    #[test]
    fn kill_by_name_from_entries_logs_success_when_no_failures() {
        init_tracing_once();
        let entries = vec![(1, "a.exe".to_string()), (2, "a.exe".to_string())];
        let out = kill_by_name_from_entries("a.exe", entries, |_pid| Ok(true)).unwrap();
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].killed, 2);
        assert_eq!(out.items[0].failed, 0);
    }

    /// `kill_by_name_from_entries`：遇到非“可忽略错误”应直接返回错误。
    #[test]
    fn kill_by_name_from_entries_propagates_unexpected_errors() {
        let err = kill_by_name_from_entries("a.exe", vec![(1, "a.exe".to_string())], |_pid| {
            Err(crate::errors::AppError::Invariant("x".to_string()))
        })
        .unwrap_err();
        assert!(matches!(err, crate::errors::AppError::Invariant(_)));
    }

    /// `kill_pid`：非 Windows 下对不存在的 PID 应返回 Ok(false)。
    #[test]
    #[cfg(not(windows))]
    fn kill_pid_returns_false_when_pid_missing() {
        assert_eq!(kill_pid(u32::MAX).unwrap(), false);
    }

    /// `eq_process_name`：非 Windows 下应大小写敏感；Windows 下应忽略大小写。
    #[test]
    #[cfg(not(windows))]
    fn eq_process_name_is_case_sensitive_on_non_windows() {
        assert!(eq_process_name("WeChat.exe", "WeChat.exe"));
        assert!(!eq_process_name("WeChat.exe", "wechat.exe"));
    }

    /// `eq_process_name`：Windows 下应忽略 ASCII 大小写。
    #[test]
    #[cfg(windows)]
    fn eq_process_name_is_case_insensitive_on_windows() {
        assert!(eq_process_name("WeChat.exe", "wechat.exe"));
        assert!(eq_process_name("WECHAT.EXE", "wechat.exe"));
    }
}
