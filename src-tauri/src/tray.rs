//! 系统托盘：最小化到托盘、托盘菜单、以及“剩余时间”动态图标。
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::Manager as _;

use crate::app_data::Phase;
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

/// 托盘菜单项 id：开始。
const MENU_START_ID: &str = "tray.start";
/// 托盘菜单项 id：暂停。
const MENU_PAUSE_ID: &str = "tray.pause";
/// 托盘菜单项 id：显示窗口。
const MENU_SHOW_ID: &str = "tray.show";
/// 托盘菜单项 id：进入迷你模式。
const MENU_MINI_ON_ID: &str = "tray.mini_on";
/// 托盘菜单项 id：退出迷你模式。
const MENU_MINI_OFF_ID: &str = "tray.mini_off";
/// 托盘菜单项 id：退出。
const MENU_QUIT_ID: &str = "tray.quit";

/// 托盘句柄集合（包含 `TrayIcon` 与需要动态更新的菜单项）。
#[derive(Clone)]
pub struct TrayHandles {
    /// 托盘图标句柄。
    pub tray: TrayIcon<tauri::Wry>,
    /// “开始”菜单项。
    pub start_item: MenuItem<tauri::Wry>,
    /// “暂停”菜单项。
    pub pause_item: MenuItem<tauri::Wry>,
    /// “进入迷你模式”菜单项。
    pub mini_on_item: MenuItem<tauri::Wry>,
    /// “退出迷你模式”菜单项。
    pub mini_off_item: MenuItem<tauri::Wry>,
}

/// 创建托盘与菜单，并写入 `AppState`。
pub fn setup_tray(app: &mut tauri::App) -> AppResult<()> {
    let state = app.state::<AppState>();
    let snapshot = state.timer_snapshot();
    let initial_text = format_mm_ss(snapshot.remaining_seconds);
    let window_mode = state.window_mode_snapshot();

    let menu = Menu::new(app)?;
    let start_item = MenuItem::with_id(app, MENU_START_ID, "开始", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, MENU_PAUSE_ID, "暂停", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, MENU_SHOW_ID, "显示窗口", true, None::<&str>)?;
    let mini_on_item = MenuItem::with_id(
        app,
        MENU_MINI_ON_ID,
        "进入迷你模式",
        !window_mode.mini_mode,
        None::<&str>,
    )?;
    let mini_off_item = MenuItem::with_id(
        app,
        MENU_MINI_OFF_ID,
        "退出迷你模式",
        window_mode.mini_mode,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT_ID, "退出", true, None::<&str>)?;
    menu.append_items(&[
        &start_item,
        &pause_item,
        &show_item,
        &mini_on_item,
        &mini_off_item,
        &quit_item,
    ])?;

    let initial_icon = build_tray_icon_rgba(&initial_text, snapshot.phase, snapshot.is_running)?;
    let tray = TrayIconBuilder::new()
        .menu(&menu)
        // 禁用“左键显示托盘菜单”：避免左键点击时系统菜单闪现（我们仅在左键时显示主窗口）。
        .show_menu_on_left_click(false)
        .icon(Image::new_owned(initial_icon, 32, 32))
        .tooltip("番茄钟")
        .on_menu_event(|app_handle, event| {
            let state = app_handle.state::<AppState>();
            let id = event.id().as_ref();

            match id {
                MENU_START_ID => {
                    let _ = crate::ipc::timer::timer_start_inner(&state);
                }
                MENU_PAUSE_ID => {
                    let _ = crate::ipc::timer::timer_pause_inner(&state);
                }
                MENU_SHOW_ID => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                MENU_MINI_ON_ID => {
                    let _ = crate::ipc::window::set_mini_mode_inner(app_handle, &state, true);
                }
                MENU_MINI_OFF_ID => {
                    let _ = crate::ipc::window::set_mini_mode_inner(app_handle, &state, false);
                }
                MENU_QUIT_ID => {
                    let _ = state.record_quit_interruption_before_exit();
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // 仅在“左键单击”时显示窗口：避免右键弹出菜单时也触发显示，导致菜单与窗口同时出现。
            if let tauri::tray::TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == tauri::tray::MouseButton::Left
                    && button_state == tauri::tray::MouseButtonState::Up
                {
                    if let Some(window) = tray.app_handle().get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    state.set_tray(TrayHandles {
        tray: tray.clone(),
        start_item: start_item.clone(),
        pause_item: pause_item.clone(),
        mini_on_item: mini_on_item.clone(),
        mini_off_item: mini_off_item.clone(),
    });

    let _ = refresh_tray(&state);
    Ok(())
}

/// 刷新托盘显示（图标与菜单启用状态）。
pub fn refresh_tray(state: &AppState) -> AppResult<()> {
    let Some(handles) = state.tray() else {
        return Ok(());
    };
    let snapshot = state.timer_snapshot();
    let window_mode = state.window_mode_snapshot();

    let text = format_mm_ss(snapshot.remaining_seconds);
    let rgba = build_tray_icon_rgba(&text, snapshot.phase, snapshot.is_running)?;
    handles
        .tray
        .set_icon(Some(Image::new_owned(rgba, 32, 32)))?;

    // 启用状态：运行中只能暂停；未运行只能开始。
    let _ = handles.start_item.set_enabled(!snapshot.is_running);
    let _ = handles.pause_item.set_enabled(snapshot.is_running);
    let _ = handles.mini_on_item.set_enabled(!window_mode.mini_mode);
    let _ = handles.mini_off_item.set_enabled(window_mode.mini_mode);

    Ok(())
}

/// 将秒数格式化为 `mm:ss`。
fn format_mm_ss(seconds: u64) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}", m.min(99), s)
}

/// 构建托盘图标 RGBA（32x32），用 7 段数码管样式绘制 `mm:ss`。
fn build_tray_icon_rgba(text: &str, phase: Phase, is_running: bool) -> AppResult<Vec<u8>> {
    if text.len() != 5 || text.as_bytes()[2] != b':' {
        return Err(AppError::Invariant("托盘时间文本必须为 mm:ss".to_string()));
    }

    let bg = match phase {
        Phase::Work => [255u8, 59u8, 48u8, 255u8],
        Phase::ShortBreak => [52u8, 199u8, 89u8, 255u8],
        Phase::LongBreak => [0u8, 122u8, 255u8, 255u8],
    };
    let fg = if is_running {
        [255u8, 255u8, 255u8, 255u8]
    } else {
        [255u8, 255u8, 255u8, 220u8]
    };

    let mut rgba = vec![0u8; 32 * 32 * 4];
    fill_round_rect(&mut rgba, 32, 32, 2, 2, 28, 28, 8, bg);

    let bytes = text.as_bytes();
    let d0 = bytes[0] - b'0';
    let d1 = bytes[1] - b'0';
    let d2 = bytes[3] - b'0';
    let d3 = bytes[4] - b'0';

    draw_digit(&mut rgba, 32, 32, 4, 8, d0, fg);
    draw_digit(&mut rgba, 32, 32, 11, 8, d1, fg);
    draw_colon(&mut rgba, 32, 32, 18, 10, fg);
    draw_digit(&mut rgba, 32, 32, 20, 8, d2, fg);
    draw_digit(&mut rgba, 32, 32, 27, 8, d3, fg);

    Ok(rgba)
}

/// 绘制 7 段数字（0-9）。
fn draw_digit(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, d: u8, color: [u8; 4]) {
    let seg = digit_segments(d);

    // 每段用小矩形表示（坐标相对 digit 原点）。
    if seg & 0b0000001 != 0 {
        fill_rect(buf, w, h, x + 1, y, 4, 1, color); // a
    }
    if seg & 0b0000010 != 0 {
        fill_rect(buf, w, h, x + 5, y + 1, 1, 3, color); // b
    }
    if seg & 0b0000100 != 0 {
        fill_rect(buf, w, h, x + 5, y + 5, 1, 3, color); // c
    }
    if seg & 0b0001000 != 0 {
        fill_rect(buf, w, h, x + 1, y + 8, 4, 1, color); // d
    }
    if seg & 0b0010000 != 0 {
        fill_rect(buf, w, h, x, y + 5, 1, 3, color); // e
    }
    if seg & 0b0100000 != 0 {
        fill_rect(buf, w, h, x, y + 1, 1, 3, color); // f
    }
    if seg & 0b1000000 != 0 {
        fill_rect(buf, w, h, x + 1, y + 4, 4, 1, color); // g
    }
}

/// 绘制冒号。
fn draw_colon(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, color: [u8; 4]) {
    fill_rect(buf, w, h, x, y, 2, 2, color);
    fill_rect(buf, w, h, x, y + 6, 2, 2, color);
}

/// 数字到 7 段（abcdefg）映射，返回位图。
fn digit_segments(d: u8) -> u8 {
    match d {
        0 => 0b0111111,
        1 => 0b0000110,
        2 => 0b1011011,
        3 => 0b1001111,
        4 => 0b1100110,
        5 => 0b1101101,
        6 => 0b1111101,
        7 => 0b0000111,
        8 => 0b1111111,
        9 => 0b1101111,
        _ => 0b0000000,
    }
}

/// 填充矩形（RGBA 覆盖）。
#[allow(clippy::too_many_arguments)]
fn fill_rect(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, rw: i32, rh: i32, c: [u8; 4]) {
    for yy in y.max(0)..(y + rh).min(h as i32) {
        for xx in x.max(0)..(x + rw).min(w as i32) {
            let idx = ((yy as u32 * w + xx as u32) * 4) as usize;
            buf[idx..idx + 4].copy_from_slice(&c);
        }
    }
}

/// 填充圆角矩形（简单圆角裁切）。
#[allow(clippy::too_many_arguments)]
fn fill_round_rect(
    buf: &mut [u8],
    w: u32,
    h: u32,
    x: i32,
    y: i32,
    rw: i32,
    rh: i32,
    r: i32,
    c: [u8; 4],
) {
    for yy in y.max(0)..(y + rh).min(h as i32) {
        for xx in x.max(0)..(x + rw).min(w as i32) {
            let inside = is_inside_rounded_rect(xx - x, yy - y, rw, rh, r);
            if inside {
                let idx = ((yy as u32 * w + xx as u32) * 4) as usize;
                buf[idx..idx + 4].copy_from_slice(&c);
            }
        }
    }
}

/// 判断像素点是否在圆角矩形内。
fn is_inside_rounded_rect(px: i32, py: i32, w: i32, h: i32, r: i32) -> bool {
    let rx = px.clamp(0, w - 1);
    let ry = py.clamp(0, h - 1);

    // 中间矩形区域
    if (r..(w - r)).contains(&rx) || (r..(h - r)).contains(&ry) {
        return true;
    }

    // 四个角做圆形裁剪
    let (cx, cy) = if rx < r && ry < r {
        (r, r)
    } else if rx >= w - r && ry < r {
        (w - r - 1, r)
    } else if rx < r && ry >= h - r {
        (r, h - r - 1)
    } else {
        (w - r - 1, h - r - 1)
    };

    let dx = rx - cx;
    let dy = ry - cy;
    dx * dx + dy * dy <= r * r
}
