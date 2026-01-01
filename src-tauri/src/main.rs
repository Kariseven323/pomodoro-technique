// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 应用二进制入口：转交给库入口 `tauri_app_lib::run()`。
fn main() {
    tauri_app_lib::run()
}
