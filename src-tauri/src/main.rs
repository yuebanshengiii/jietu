#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::OnceLock;
use tauri::{Emitter, Manager, Runtime, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

mod ocr;
mod translate;
mod hotkey;
mod capture;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            APP_HANDLE.set(app.handle().clone()).ok();
            
            // 注册全局热键
            let handle = app.handle();
            handle.global_shortcut().on_shortcut("capture-area", |_app, _shortcut, _event| {
                trigger_area_capture(handle.clone());
            })?;
            
            // 隐藏主窗口
            if let Some(main) = handle.get_webview_window("main") {
                main.hide().ok();
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ocr::recognize_image,
            translate::translate_text,
            hotkey::show_selector,
            hotkey::hide_selector,
            hotkey::show_tooltip,
            hotkey::hide_tooltip,
            capture::capture_area,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn trigger_area_capture(handle: tauri::AppHandle) {
    if let Some(selector) = handle.get_webview_window("selector") {
        selector.show().ok();
        selector.set_focus().ok();
        // 发送事件给前端开始框选
        selector.emit("start-selection", ()).ok();
    }
}

pub fn get_app_handle() -> Option<&'static tauri::AppHandle> {
    APP_HANDLE.get()
}