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
            return Err(AppError::Invariant("提取图标失败（SHGetFileInfoW 返回 0）".to_string()));
        }

        let hicon = shfi.hIcon;
        if hicon.0.is_null() {
            return Err(AppError::Invariant("提取图标失败（HICON 为空）".to_string()));
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
            return Err(AppError::Invariant("CreateDIBSection 失败（句柄/像素指针为空）".to_string()));
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
