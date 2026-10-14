use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Deserialize)]
pub struct CaptureRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[cfg(target_os = "windows")]
mod windows_capture {
    use super::*;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BitBlt, SRCCOPY, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    pub fn capture_area(x: i32, y: i32, width: u32, height: u32) -> Result<String> {
        unsafe {
            let hwnd = GetDesktopWindow();
            let hdc_screen = GetDC(Some(hwnd));
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbitmap = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
            let old_bitmap = SelectObject(hdc_mem, hbitmap);
            
            BitBlt(hdc_mem, 0, 0, width as i32, height as i32, hdc_screen, x, y, SRCCOPY);
            
            // 获取位图数据
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width as i32;
            bmi.bmiHeader.biHeight = -(height as i32); // 自上而下
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;
            
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            GetDIBits(hdc_mem, hbitmap, 0, height, Some(pixels.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
            
            // 清理
            SelectObject(hdc_mem, old_bitmap);
            DeleteObject(hbitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);
            
            // 转 PNG base64
            let png_data = encode_png(&pixels, width, height)?;
            Ok(general_purpose::STANDARD.encode(png_data))
        }
    }
    
    fn encode_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| anyhow::anyhow!("创建图像失败"))?;
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;
        Ok(buf)
    }
}

#[cfg(not(target_os = "windows"))]
mod fallback_capture {
    use super::*;
    pub fn capture_area(_x: i32, _y: i32, _width: u32, _height: u32) -> Result<String> {
        Err(anyhow::anyhow!("非 Windows 平台暂不支持截图"))
    }
}

#[command]
pub async fn capture_area(request: CaptureRequest) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        windows_capture::capture_area(request.x, request.y, request.width, request.height)
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        fallback_capture::capture_area(request.x, request.y, request.width, request.height)
            .map_err(|e| e.to_string())
    }
}