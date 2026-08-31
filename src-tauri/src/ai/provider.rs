//! OpenAI 兼容 chat/completions 客户端：SSE 流式 + 非流式 + 连通性自检。
//! 网络在 async runtime；解析类为纯函数可单测。

use serde::Deserialize;
use serde_json::{json, Value};

use super::actions::{AiAction, AiInputKind};
use super::prompt::{compose_image_prompt, compose_prompt};
use super::{AiError, TemplateCtx};

/// 流式请求超时（秒）。
const STREAM_TIMEOUT_SECS: u64 = 120;

/// 连通性自检超时（秒）。
const CONNECT_TIMEOUT_SECS: u64 = 15;

/// 默认采样温度。
const TEMPERATURE: f64 = 0.7;

/// 连通性自检 max_tokens。
const CONNECT_MAX_TOKENS: u32 = 8;

/// 正式请求输出 max_tokens：截断失控模型（如 OCR 模型对 UI 截图
/// 无限重复倾倒文字），避免烧穿用户额度；翻译/润色等任务 4096 tokens 足够。
const MAX_OUTPUT_TOKENS: u32 = 4096;

/// 流式响应累计文本上限（字符数），超过即截断并报错。
const MAX_RESPONSE_CHARS: usize = 100_000;

/// HTTP 错误响应体截取长度。
const ERROR_BODY_CHAR_LIMIT: usize = 200;

/// 校验 base_url scheme：只允许 http(s)。
/// 防止 file:// 等 scheme 读取本地文件（SSRF 变体）。
fn validate_base_url(base_url: &str) -> Result<(), AiError> {
    let trimmed = base_url.trim();

    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err(AiError::Provider(anyhow::anyhow!(
            "baseUrl 只支持 http 或 https"
        )));
    }

    Ok(())
}

/// 构造 chat/completions 完整 URL。
fn build_chat_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

/// 格式化 HTTP 错误响应。
fn format_http_error(status: reqwest::StatusCode, body: String) -> AiError {
    AiError::Provider(anyhow::anyhow!(
        "HTTP {status}: {}",
        body.chars().take(ERROR_BODY_CHAR_LIMIT).collect::<String>()
    ))
}

/// 构建复用的 reqwest Client（连接池共享）。
fn build_client(timeout_secs: u64) -> Result<reqwest::Client, AiError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| AiError::Provider(e.into()))
}

/// Provider 连接配置（来自 Settings.ai）。
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// 把动作提示词与自动输入组装为一条 user message。
/// 文本动作附加纯文本输入；图片动作使用多模态 content，图片始终作为 image_url
/// 传递，不把 base64 拼入普通提示词。
pub fn build_messages(action: &AiAction, ctx: &TemplateCtx) -> Result<Vec<Value>, AiError> {
    let user_content = match action.input_kind {
        AiInputKind::Text => json!(compose_prompt(&action.prompt, ctx.content.as_deref())?),
        AiInputKind::Image => {
            let image = ctx.image.as_deref().ok_or(AiError::EmptyContent)?;

            json!([
                {"type": "text", "text": compose_image_prompt(&action.prompt)?},
                {"type": "image_url", "image_url": {"url": image}},
            ])
        }
    };

    Ok(vec![json!({"role": "user", "content": user_content})])
}

/// 单行 SSE 解析：提取 `data:` 后 JSON 的 `choices[0].delta.content`。
pub fn parse_sse_delta(line: &str) -> Option<String> {
    let payload = line.strip_prefix("data:")?;
    let payload = payload.trim_start();

    if payload.is_empty() || payload == "[DONE]" || payload.starts_with(':') {
        return None;
    }

    #[derive(Deserialize)]
    struct SseEnvelope<'a> {
        #[serde(borrow)]
        choices: Option<Vec<SseChoice<'a>>>,
    }
    #[derive(Deserialize)]
    struct SseChoice<'a> {
        #[serde(borrow)]
        delta: SseDelta<'a>,
    }
    #[derive(Deserialize)]
    struct SseDelta<'a> {
        #[serde(borrow, default)]
        content: Option<&'a str>,
    }

    let Ok(envelope) = serde_json::from_str::<SseEnvelope>(payload) else {
        return None;
    };

    envelope
        .choices?
        .into_iter()
        .next()?
        .delta
        .content
        .map(str::to_owned)
}

/// 检查 SSE 累积响应是否超过字符上限。
/// 使用字节长度做近似判断（UTF-8 下字节数 ≥ 字符数），避免每次 O(n) 全量遍历。
/// 超限时返回错误，否则返回更新后的字符计数。
pub fn check_response_limit(full_len: usize, max_chars: usize) -> Result<(), AiError> {
    if full_len > max_chars {
        return Err(AiError::Provider(anyhow::anyhow!(
            "AI 响应超过 {max_chars} 字符上限"
        )));
    }

    Ok(())
}

pub async fn stream_chat(
    config: &ProviderConfig,
    messages: Vec<Value>,
    mut on_delta: impl FnMut(String) + Send,
) -> Result<String, AiError> {
    validate_base_url(&config.base_url)?;

    let client = build_client(STREAM_TIMEOUT_SECS)?;
    let url = build_chat_url(&config.base_url);

    let body = json!({
        "model": config.model,
        "messages": messages,
        "temperature": TEMPERATURE,
        "max_tokens": MAX_OUTPUT_TOKENS,
        "stream": true,
    });

    let mut response = client
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(map_reqwest_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        return Err(format_http_error(status, text));
    }

    let mut full = String::new();
    let mut buffer = String::new();

    loop {
        // 读流中途出错必须上抛：静默断流会让用户拿到被截断的"成功"结果。
        let chunk = response.chunk().await.map_err(map_reqwest_error)?;

        let Some(bytes) = chunk else {
            break;
        };

        let text = String::from_utf8_lossy(&bytes);

        buffer.push_str(&text);

        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();

            if let Some(delta) = parse_sse_delta(line.trim_end()) {
                full.push_str(&delta);

                check_response_limit(full.len(), MAX_RESPONSE_CHARS)?;
                on_delta(delta);
            }
        }
    }

    Ok(full.trim_end().to_owned())
}

/// 把 reqwest 错误翻译为用户可读文案；超时单独提示（部分模型仅支持流式，
/// 非流式会被服务端挂起直到超时）。
pub fn map_reqwest_error(error: reqwest::Error) -> AiError {
    if error.is_timeout() {
        return AiError::Provider(anyhow::anyhow!(
            "AI 服务响应超时。若当前模型档案关闭了流式输出，部分模型仅支持流式，请开启流式后重试"
        ));
    }

    AiError::Provider(error.into())
}

/// 发送非流式 chat/completions，返回 `choices[0].message.content` 全文。
pub async fn chat_once(config: &ProviderConfig, messages: Vec<Value>) -> Result<String, AiError> {
    let value = post_chat(
        config,
        messages,
        Some(MAX_OUTPUT_TOKENS),
        STREAM_TIMEOUT_SECS,
    )
    .await?;

    Ok(value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .trim_end()
        .to_owned())
}

/// 连通性自检：发送一条极短请求（stream=false），返回模型名或自身文案。
pub async fn check_connectivity(config: &ProviderConfig) -> Result<String, AiError> {
    let value = post_chat(
        config,
        build_ping_messages(),
        Some(CONNECT_MAX_TOKENS),
        CONNECT_TIMEOUT_SECS,
    )
    .await?;

    let model = value["model"].as_str().unwrap_or(&config.model).to_owned();

    Ok(model)
}

/// 非流式 chat/completions 公共请求：可选 max_tokens，返回完整响应 JSON。
async fn post_chat(
    config: &ProviderConfig,
    messages: Vec<Value>,
    max_tokens: Option<u32>,
    timeout_secs: u64,
) -> Result<Value, AiError> {
    validate_base_url(&config.base_url)?;

    let client = build_client(timeout_secs)?;

    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "temperature": TEMPERATURE,
        "stream": false,
    });
    if let Some(max_tokens) = max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }

    let response = client
        .post(build_chat_url(&config.base_url))
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(map_reqwest_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        return Err(format_http_error(status, text));
    }

    response
        .json()
        .await
        .map_err(|e| AiError::Provider(e.into()))
}

/// 连通性自检消息体。
fn build_ping_messages() -> Vec<Value> {
    vec![json!({"role": "user", "content": "ping"})]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::actions::AiAction;
    use crate::ai::AiInputKind;

    #[test]
    fn parse_delta_line() {
        let line = r#"data: {"choices":[{"delta":{"content":"你好"}}]}"#;
        assert_eq!(parse_sse_delta(line).as_deref(), Some("你好"));
    }

    #[test]
    fn parse_done_marker() {
        assert_eq!(parse_sse_delta("data: [DONE]"), None);
    }

    #[test]
    fn parse_ignores_comment_and_empty() {
        assert_eq!(parse_sse_delta(": keep-alive"), None);
        assert_eq!(parse_sse_delta(""), None);
    }

    #[test]
    fn parse_skips_malformed() {
        let usage = r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;
        assert_eq!(parse_sse_delta(usage), None);
        assert_eq!(parse_sse_delta("data: not-json"), None);
    }

    #[test]
    fn build_multimodal_messages() {
        let action = AiAction {
            id: "ocr".into(),
            input_kind: AiInputKind::Image,
            label_key: "ocr".into(),
            prompt: "识别图片中的文字".into(),
            model_profile_id: None,
        };
        let ctx = TemplateCtx {
            content: None,
            image: Some("data:image/png;base64,AA==".into()),
        };

        let msgs = build_messages(&action, &ctx).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "识别图片中的文字");
        assert_eq!(content[1]["type"], "image_url");
    }

    #[test]
    fn build_text_message_contains_prompt_and_content() {
        let action = AiAction {
            id: "translate".into(),
            input_kind: AiInputKind::Text,
            label_key: "翻译".into(),
            prompt: "翻译为中文".into(),
            model_profile_id: None,
        };
        let ctx = TemplateCtx {
            content: Some("hello".into()),
            image: None,
        };

        let msgs = build_messages(&action, &ctx).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert!(msgs[0]["content"].as_str().unwrap().contains("翻译为中文"));
        assert!(msgs[0]["content"].as_str().unwrap().contains("hello"));
    }

    #[test]
    fn validate_base_url_accepts_https() {
        assert!(validate_base_url("https://api.openai.com").is_ok());
    }

    #[test]
    fn validate_base_url_accepts_http() {
        assert!(validate_base_url("http://localhost:8080").is_ok());
    }

    #[test]
    fn validate_base_url_rejects_file_scheme() {
        assert!(validate_base_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn validate_base_url_rejects_ftp_scheme() {
        assert!(validate_base_url("ftp://malicious.com").is_err());
    }

    #[test]
    fn validate_base_url_rejects_empty() {
        assert!(validate_base_url("").is_err());
        assert!(validate_base_url("   ").is_err());
    }

    #[test]
    fn check_response_limit_allows_under_max() {
        assert!(check_response_limit(100, 100_000).is_ok());
    }

    #[test]
    fn check_response_limit_rejects_over_max() {
        assert!(check_response_limit(100_001, 100_000).is_err());
    }
}
