//! Windows 进程列表与终止逻辑（专注模式核心能力）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sysinfo::System;

#[cfg(windows)]
use crate::errors::AppError;
use crate::errors::AppResult;

/// 向前端广播“终止黑名单进程结果”的事件名。
pub const EVENT_KILL_RESULT: &str = "pomodoro://kill_result";

/// 前端展示用的进程信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    /// 进程名（例如 `WeChat.exe`）。
    pub name: String,
    /// 代表性 PID（用于展示）。
    pub pid: u32,
    /// 可执行文件路径（若可获取）。
    pub exe_path: Option<String>,
    /// 进程图标（data URL：`data:image/png;base64,...`）。
    pub icon_data_url: Option<String>,
}

/// 单个进程名的终止结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KillSummary {
    /// 各进程名的明细。
    pub items: Vec<KillItem>,
    /// 是否有任何条目需要管理员权限。
    pub requires_admin: bool,
}

/// 获取当前运行进程列表（按进程名去重并按名称排序）。
pub fn list_processes() -> AppResult<Vec<ProcessInfo>> {
    let mut system = System::new_all();
    system.refresh_all();

    let mut by_name: BTreeMap<String, ProcessInfo> = BTreeMap::new();

    for (pid, process) in system.processes() {
        let name = process.name().to_string();
        let pid_u32 = pid.as_u32();
        let exe_path = process.exe().and_then(|p| {
            if p.as_os_str().is_empty() {
                None
            } else {
                Some(p.to_string_lossy().to_string())
            }
        });

        by_name.entry(name.clone()).or_insert_with(|| ProcessInfo {
            name,
            pid: pid_u32,
            exe_path: exe_path.clone(),
            icon_data_url: exe_path
                .as_deref()
                .and_then(|p| icon_data_url_best_effort(p).ok().flatten()),
        });
    }

    Ok(by_name.into_values().collect())
}

/// 终止所有匹配 `process_name` 的进程（返回可用于 UI 展示的汇总结果）。
pub fn kill_by_name(process_name: &str) -> AppResult<KillSummary> {
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
        tracing::info!(target: "blacklist", "终止进程成功：name={} killed={}", process_name, killed);
    }

    Ok(KillSummary {
        items,
        requires_admin: any_requires_admin,
    })
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

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
            AppError::KillFailed(format!(
                "OpenProcess 失败（{e:?}）ACCESS_DENIED 可能需要管理员权限"
            ))
        })?;

        let result = TerminateProcess(handle, 1).map(|_| true).map_err(|e| {
            AppError::KillFailed(format!(
                "TerminateProcess 失败（{e:?}）ACCESS_DENIED 可能需要管理员权限"
            ))
        });

        let _ = CloseHandle(handle);
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

/// 将可执行文件路径转为图标 data URL（失败时返回 `Ok(None)`）。
fn icon_data_url_best_effort(exe_path: &str) -> AppResult<Option<String>> {
    #[cfg(windows)]
    {
        use base64::Engine as _;
        use std::path::Path;
        let png = extract_exe_icon_png(Path::new(exe_path), 32)?;
        Ok(Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )))
    }

    #[cfg(not(windows))]
    {
        let _ = exe_path;
        Ok(None)
    }
}

/// Windows：从 exe 路径提取小图标并编码为 PNG（32x32）。
#[cfg(windows)]
fn extract_exe_icon_png(exe_path: &std::path::Path, size: i32) -> AppResult<Vec<u8>> {
    use image::{ImageBuffer, Rgba};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HBRUSH, HDC, HGDIOBJ,
        RGBQUAD,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

    let wide = to_wide_null(exe_path.to_string_lossy().as_ref());
    let mut shfi = SHFILEINFOW::default();

    unsafe {
        let ok = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi as *mut SHFILEINFOW),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );
        if ok == 0 {
            return Err(AppError::KillFailed("提取图标失败".to_string()));
        }

        let hicon = shfi.hIcon;
        if hicon.0.is_null() {
            return Err(AppError::KillFailed("提取图标失败".to_string()));
        }

        let dc: HDC = CreateCompatibleDC(HDC(std::ptr::null_mut()));
        if dc.0.is_null() {
            let _ = DestroyIcon(hicon);
            return Err(AppError::KillFailed("CreateCompatibleDC 失败".to_string()));
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp: HBITMAP = CreateDIBSection(
            dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            HANDLE(std::ptr::null_mut()),
            0,
        )
        .map_err(|e| AppError::KillFailed(format!("CreateDIBSection 失败（{e:?}）")))?;

        if hbmp.0.is_null() || bits.is_null() {
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(hicon);
            return Err(AppError::KillFailed("CreateDIBSection 失败".to_string()));
        }

        let old: HGDIOBJ = SelectObject(dc, hbmp);
        let _ = DrawIconEx(
            dc,
            0,
            0,
            hicon,
            size,
            size,
            0,
            HBRUSH(std::ptr::null_mut()),
            DI_NORMAL,
        );

        let mut buf = vec![0u8; (size * size * 4) as usize];
        let scanlines = GetDIBits(
            dc,
            hbmp,
            0,
            size as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = SelectObject(dc, old);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(hicon);

        if scanlines == 0 {
            return Err(AppError::KillFailed("GetDIBits 失败".to_string()));
        }

        // Windows DIB 为 BGRA，这里转为 RGBA。
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(size as u32, size as u32, buf)
            .ok_or_else(|| AppError::Invariant("从像素缓冲构建图像失败".to_string()))?;

        let mut png: Vec<u8> = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| AppError::KillFailed(format!("PNG 编码失败：{e}")))?;

        Ok(png)
    }
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
