use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::command;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("HTTP client"))
}

#[derive(Debug, Deserialize)]
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateResult {
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
}

// LibreTranslate API
async fn translate_libre(text: &str, source: &str, target: &str) -> Result<TranslateResult> {
    #[derive(Serialize)]
    struct LibreRequest<'a> {
        q: &'a str,
        source: &'a str,
        target: &'a str,
        format: &'a str,
    }

    #[derive(Deserialize)]
    struct LibreResponse {
        translatedText: String,
    }

    // 使用公共实例，也可自建
    let url = "https://libretranslate.de/translate";
    
    let req = LibreRequest {
        q: text,
        source,
        target,
        format: "text",
    };

    let resp: LibreResponse = client()
        .post(url)
        .json(&req)
        .send()
        .await?
        .json()
        .await?;

    Ok(TranslateResult {
        original: text.to_string(),
        translated: resp.translatedText,
        source_lang: source.to_string(),
        target_lang: target.to_string(),
    })
}

// MyMemory API (备用)
async fn translate_mymemory(text: &str, source: &str, target: &str) -> Result<TranslateResult> {
    #[derive(Deserialize)]
    struct MyMemoryResponse {
        responseData: MyMemoryData,
    }

    #[derive(Deserialize)]
    struct MyMemoryData {
        translatedText: String,
    }

    let lang_pair = format!("{}|{}", source, target);
    let url = format!("https://api.mymemory.translated.net/get?q={}&langpair={}", 
        urlencoding::encode(text), lang_pair);

    let resp: MyMemoryResponse = client()
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    Ok(TranslateResult {
        original: text.to_string(),
        translated: resp.responseData.translatedText,
        source_lang: source.to_string(),
        target_lang: target.to_string(),
    })
}

#[command]
pub async fn translate_text(request: TranslateRequest) -> Result<TranslateResult, String> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err("文本为空".into());
    }

    let source = request.source_lang.unwrap_or_else(|| "en".into());
    let target = request.target_lang.unwrap_or_else(|| "zh".into());

    // 尝试 LibreTranslate
    match translate_libre(text, &source, &target).await {
        Ok(result) => return Ok(result),
        Err(e) => log::warn!("LibreTranslate 失败: {}", e),
    }

    // 备用 MyMemory
    match translate_mymemory(text, &source, &target).await {
        Ok(result) => return Ok(result),
        Err(e) => return Err(format!("翻译失败: {}", e)),
    }
}