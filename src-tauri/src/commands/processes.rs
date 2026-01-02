//! 进程相关命令：列举进程、提权重启等。

use crate::errors::{AppError, AppResult};
use crate::processes::{self, ProcessInfo};

use super::common::to_ipc_result;

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

/// 提权重启的内部实现（便于统一错误处理）。
fn restart_as_admin_impl() -> AppResult<()> {
    #[cfg(windows)]
    {
        restart_as_admin_windows()
    }
    #[cfg(not(windows))]
    {
        Err(AppError::UnsupportedPlatform(
            "仅 Windows 支持“以管理员身份重启”".to_string(),
        ))
    }
}

/// Windows：通过 `ShellExecuteW` 使用 `runas` 重新启动自身并退出。
#[cfg(windows)]
fn restart_as_admin_windows() -> AppResult<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe()
        .map_err(|e| AppError::Invariant(format!("获取当前可执行文件失败：{e}")))?;
    let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let exe_w = to_wide_null(exe.to_string_lossy().as_ref());
    let verb_w = to_wide_null("runas");
    let args_w = to_wide_null(&args);

    unsafe {
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(verb_w.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(args_w.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        let code = result.0 as isize;
        if code <= 32 {
            return Err(AppError::Invariant(format!(
                "提权重启失败（ShellExecute 返回 {code:?}）",
            )));
        }
    }

    std::process::exit(0);
}

/// Windows：将 `&str` 转为以 `\\0` 结尾的 UTF-16。
#[cfg(windows)]
fn to_wide_null(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}
