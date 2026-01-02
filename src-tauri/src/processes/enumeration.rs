//! 进程枚举与图标提取（用于黑名单管理 UI）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use ts_rs::TS;

#[cfg(windows)]
use crate::errors::AppError;
use crate::errors::AppResult;

/// 前端展示用的进程信息。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
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

/// 内部用的“进程快照”条目（用于将 sysinfo 结果转为可测试的纯数据流）。
#[derive(Debug, Clone)]
struct ProcessEntry {
    /// 进程名。
    name: String,
    /// PID。
    pid: u32,
    /// 可执行文件路径。
    exe_path: Option<String>,
}

/// 获取当前运行进程列表（按进程名去重并按名称排序）。
pub fn list_processes() -> AppResult<Vec<ProcessInfo>> {
    let mut system = System::new_all();
    system.refresh_all();

    let entries = system.processes().iter().map(|(pid, process)| {
        let exe_path = process.exe().and_then(normalize_sysinfo_exe_path);
        ProcessEntry {
            name: process.name().to_string(),
            pid: pid.as_u32(),
            exe_path,
        }
    });

    Ok(list_processes_from_entries(entries))
}

/// 将 sysinfo 返回的 exe 路径规范化为可序列化字符串（空路径视为 None）。
fn normalize_sysinfo_exe_path(path: &std::path::Path) -> Option<String> {
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path.to_string_lossy().to_string())
    }
}

/// 将“进程快照”条目列表转为前端展示用 `ProcessInfo`（去重、排序、图标 best-effort）。
fn list_processes_from_entries(
    entries: impl IntoIterator<Item = ProcessEntry>,
) -> Vec<ProcessInfo> {
    let mut by_name: BTreeMap<String, ProcessInfo> = BTreeMap::new();

    for entry in entries {
        let exe_path = entry
            .exe_path
            .and_then(|p| if p.trim().is_empty() { None } else { Some(p) });

        by_name
            .entry(entry.name.clone())
            .or_insert_with(|| ProcessInfo {
                name: entry.name,
                pid: entry.pid,
                exe_path: exe_path.clone(),
                icon_data_url: exe_path
                    .as_deref()
                    .and_then(|p| icon_data_url_best_effort(p).ok().flatten()),
            });
    }

    by_name.into_values().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `list_processes_from_entries`：应按名称去重并按名称排序。
    #[test]
    fn list_processes_dedupes_and_sorts_by_name() {
        let entries = vec![
            ProcessEntry {
                name: "b.exe".to_string(),
                pid: 2,
                exe_path: Some("/bin/b".to_string()),
            },
            ProcessEntry {
                name: "a.exe".to_string(),
                pid: 1,
                exe_path: Some("/bin/a".to_string()),
            },
            // 重复名称：应保留第一次插入的 pid/exe_path
            ProcessEntry {
                name: "a.exe".to_string(),
                pid: 999,
                exe_path: Some("/bin/a2".to_string()),
            },
        ];

        let out = list_processes_from_entries(entries);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "a.exe");
        assert_eq!(out[0].pid, 1);
        assert_eq!(out[0].exe_path.as_deref(), Some("/bin/a"));
        assert_eq!(out[1].name, "b.exe");
    }

    /// `list_processes_from_entries`：空 exe_path 应规范化为 None，避免 UI 展示异常。
    #[test]
    fn list_processes_normalizes_empty_exe_path() {
        let entries = vec![ProcessEntry {
            name: "a.exe".to_string(),
            pid: 1,
            exe_path: Some("".to_string()),
        }];

        let out = list_processes_from_entries(entries);
        assert_eq!(out.len(), 1);
        assert!(out[0].exe_path.is_none());
        #[cfg(not(windows))]
        assert!(out[0].icon_data_url.is_none());
    }

    /// `icon_data_url_best_effort`：非 Windows 下应始终返回 Ok(None)。
    #[test]
    #[cfg(not(windows))]
    fn icon_data_url_best_effort_returns_none_on_non_windows() {
        let out = icon_data_url_best_effort("/bin/bash").unwrap();
        assert!(out.is_none());
    }

    /// `list_processes`：应返回按名称排序且去重的结果（不依赖具体进程集合）。
    #[test]
    fn list_processes_returns_sorted_unique_names() {
        let out = list_processes().unwrap();
        for w in out.windows(2) {
            assert!(w[0].name <= w[1].name);
        }
        let mut seen = std::collections::BTreeSet::<String>::new();
        for p in &out {
            assert!(seen.insert(p.name.clone()));
            #[cfg(not(windows))]
            assert!(p.icon_data_url.is_none());
        }
    }

    /// `normalize_sysinfo_exe_path`：空路径应返回 None（与 sysinfo 行为对齐）。
    #[test]
    fn normalize_sysinfo_exe_path_handles_empty_path() {
        let out = normalize_sysinfo_exe_path(std::path::Path::new(""));
        assert!(out.is_none());
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

    /// 图标句柄守卫：确保 `DestroyIcon` 被调用。
    struct IconGuard(windows::Win32::UI::WindowsAndMessaging::HICON);
    impl Drop for IconGuard {
        /// 释放图标句柄。
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }

    /// DC 句柄守卫：确保 `DeleteDC` 被调用。
    struct DcGuard(HDC);
    impl Drop for DcGuard {
        /// 释放 DC。
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    /// 位图句柄守卫：确保 `DeleteObject` 被调用。
    struct BitmapGuard(HBITMAP);
    impl Drop for BitmapGuard {
        /// 释放位图句柄。
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(self.0);
            }
        }
    }

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
            return Err(AppError::Invariant(
                "提取图标失败（SHGetFileInfoW 返回 0）".to_string(),
            ));
        }

        let hicon = shfi.hIcon;
        if hicon.0.is_null() {
            return Err(AppError::Invariant(
                "提取图标失败（HICON 为空）".to_string(),
            ));
        }
        let icon = IconGuard(hicon);

        let dc: HDC = CreateCompatibleDC(HDC(std::ptr::null_mut()));
        if dc.0.is_null() {
            return Err(AppError::Invariant("CreateCompatibleDC 失败".to_string()));
        }
        let dc = DcGuard(dc);

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
            dc.0,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            HANDLE(std::ptr::null_mut()),
            0,
        )
        .map_err(|e| AppError::Invariant(format!("CreateDIBSection 失败（{e:?}）")))?;

        if hbmp.0.is_null() || bits.is_null() {
            return Err(AppError::Invariant(
                "CreateDIBSection 失败（句柄/像素指针为空）".to_string(),
            ));
        }
        let hbmp = BitmapGuard(hbmp);

        let old: HGDIOBJ = SelectObject(dc.0, hbmp.0);
        let _ = DrawIconEx(
            dc.0,
            0,
            0,
            icon.0,
            size,
            size,
            0,
            HBRUSH(std::ptr::null_mut()),
            DI_NORMAL,
        );

        let mut buf = vec![0u8; (size * size * 4) as usize];
        let scanlines = GetDIBits(
            dc.0,
            hbmp.0,
            0,
            size as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // 恢复旧对象，避免 DC 状态污染。
        let _ = SelectObject(dc.0, old);

        if scanlines == 0 {
            return Err(AppError::Invariant("GetDIBits 失败".to_string()));
        }

        // Windows DIB 为 BGRA，这里转为 RGBA。
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(size as u32, size as u32, buf)
            .ok_or_else(|| AppError::Invariant("从像素缓冲构建图像失败".to_string()))?;

        let mut png: Vec<u8> = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| AppError::Invariant(format!("PNG 编码失败：{e}")))?;

        // 通过 Drop 自动清理：hbmp/dc/icon。
        let _ = (hbmp, dc, icon);

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
