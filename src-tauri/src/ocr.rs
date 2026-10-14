use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use image::{ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
}

#[derive(Debug, Deserialize)]
pub struct OcrRequest {
    pub image_base64: String,
}

// Windows 原生 OCR (WinRT)
#[cfg(target_os = "windows")]
mod windows_ocr {
    use super::*;
    use windows::Media::Ocr::OcrEngine;
    use windows::Graphics::Imaging::{BitmapDecoder, SoftwareBitmap, BitmapPixelFormat, BitmapAlphaMode};
    use windows::Storage::Streams::{InMemoryRandomAccessStream, DataWriter};
    use windows::Foundation::TypedEventHandler;

    pub async fn recognize(data: &[u8]) -> Result<OcrResult> {
        // 将字节转为 SoftwareBitmap
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(data)?;
        writer.StoreAsync()?.await?;
        writer.FlushAsync()?.await?;
        stream.Seek(0)?;

        let decoder = BitmapDecoder::CreateAsync(&stream)?.await?;
        let software_bitmap = decoder.GetSoftwareBitmapAsync()?.await?;

        // 确保格式兼容
        let software_bitmap = SoftwareBitmap::Convert(&software_bitmap, BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied)?;

        // 使用 Windows OCR 引擎
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let result = engine.RecognizeAsync(&software_bitmap)?.await?;

        let text = result.Text()?.to_string();
        let confidence = 0.95; // Windows OCR 不直接给置信度，给个默认高值

        Ok(OcrResult { text, confidence })
    }
}

// Tesseract 兜底 (跨平台)
#[cfg(feature = "tesseract")]
mod tesseract_ocr {
    use super::*;
    use tesseract::Tesseract;

    pub fn recognize(data: &[u8]) -> Result<OcrResult> {
        let mut api = Tesseract::new(None, "eng+chi_sim")?;
        api.set_image_from_mem(data)?;
        let text = api.get_text()?;
        Ok(OcrResult { text, confidence: 0.85 })
    }
}

#[command]
pub async fn recognize_image(request: OcrRequest) -> Result<OcrResult, String> {
    let image_data = general_purpose::STANDARD.decode(&request.image_base64)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;

    // 优先尝试 Windows 原生 OCR
    #[cfg(target_os = "windows")]
    {
        match windows_ocr::recognize(&image_data).await {
            Ok(result) if !result.text.trim().is_empty() => return Ok(result),
            Ok(_) => {} // 空结果，尝试兜底
            Err(e) => log::warn!("Windows OCR 失败: {}", e),
        }
    }

    // 兜底：Tesseract (需启用 feature)
    #[cfg(feature = "tesseract")]
    {
        match tesseract_ocr::recognize(&image_data) {
            Ok(result) => return Ok(result),
            Err(e) => return Err(format!("Tesseract OCR 失败: {}", e)),
        }
    }

    // 无可用 OCR
    #[cfg(not(any(target_os = "windows", feature = "tesseract")))]
    {
        return Err("当前平台无可用 OCR 引擎".into());
    }

    Ok(OcrResult { text: String::new(), confidence: 0.0 })
}