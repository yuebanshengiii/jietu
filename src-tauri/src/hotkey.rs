use tauri::{command, Manager, WebviewWindow};

#[command]
pub fn show_selector() -> Result<(), String> {
    if let Some(handle) = crate::get_app_handle() {
        if let Some(win) = handle.get_webview_window("selector") {
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
            win.emit("start-selection", ()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[command]
pub fn hide_selector() -> Result<(), String> {
    if let Some(handle) = crate::get_app_handle() {
        if let Some(win) = handle.get_webview_window("selector") {
            win.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[command]
pub fn show_tooltip(x: f64, y: f64, original: String, translated: String) -> Result<(), String> {
    if let Some(handle) = crate::get_app_handle() {
        if let Some(win) = handle.get_webview_window("tooltip") {
            // 设置位置
            let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: x as i32,
                y: y as i32,
            }));
            
            // 发送内容
            win.emit("show-content", serde_json::json!({
                "original": original,
                "translated": translated
            })).map_err(|e| e.to_string())?;
            
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[command]
pub fn hide_tooltip() -> Result<(), String> {
    if let Some(handle) = crate::get_app_handle() {
        if let Some(win) = handle.get_webview_window("tooltip") {
            win.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}