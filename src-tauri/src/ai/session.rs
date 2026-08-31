//! 运行中 AI 请求会话表。命令层在其上 spawn/cancel，事件由请求任务自身 emit。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::json;
use tauri::Emitter;

use super::actions::AiAction;
use super::provider::ProviderConfig;
use super::{events, AiError, TemplateCtx};
use crate::clipboard::ImageStore;
use crate::db::models::{ClipboardItem, ClipboardKind};

#[derive(Default, Clone)]
pub struct AiSessions {
    inner: Arc<Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>>,
}

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// 生成本进程唯一递增 request_id（多窗口并发隔离依赖它）。
pub fn next_request_id() -> String {
    let n = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);

    format!("req-{}-{}", std::process::id(), n)
}

impl AiSessions {
    /// 登记一个运行中请求，返回 request_id。
    #[allow(dead_code)]
    pub fn insert(&self, handle: tauri::async_runtime::JoinHandle<()>) -> String {
        let id = next_request_id();
        let mut g = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        g.insert(id.clone(), handle);
        id
    }

    /// 用指定 request_id 登记运行中请求。
    pub fn insert_with_id(&self, id: &str, handle: tauri::async_runtime::JoinHandle<()>) {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.to_owned(), handle);
    }

    /// 取消并移除；返回是否存在。
    pub fn cancel(&self, id: &str) -> bool {
        let handle = self
            .inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(id);

        if let Some(h) = handle {
            h.abort();
            true
        } else {
            false
        }
    }

    /// 任务结束（emit done/error 后）自清除。
    pub fn remove(&self, id: &str) {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(id);
    }
}

/// 文本内容截断上限（字符数）。
const MAX_TEXT_CHARS: usize = 10_000;

/// 截断时追加的显式标记，让模型明确知道内容不完整。
const TRUNCATION_MARKER: &str = "\n\n……[提示：内容过长，已截断]";

/// 图片大小上限（字节）。
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// 组装请求上下文。文本条目取 `search_text`（采集时 OS 提供的纯文本表示，与
/// 「粘贴为纯文本」同源，缺失时退回 `content`；
/// 图片条目转 base64 data URL。
pub async fn build_request_ctx(
    item: &ClipboardItem,
    image_store: &ImageStore,
) -> Result<TemplateCtx, AiError> {
    let content = if item.kind == ClipboardKind::Text {
        let plain = item
            .search_text
            .clone()
            .unwrap_or_else(|| item.content.clone());

        Some(truncate_chars(&plain, MAX_TEXT_CHARS))
    } else {
        None
    };

    let image = if item.kind == ClipboardKind::Image {
        let file_name = &item.content;

        // 路径穿越防护
        let unsafe_name = file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name.contains("..");
        if unsafe_name {
            return Err(AiError::ImageUnavailable);
        }

        let path = image_store.origin_path(file_name);
        if !path.exists() {
            return Err(AiError::ImageUnavailable);
        }

        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|_| AiError::ImageUnavailable)?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(AiError::ImageUnavailable);
        }

        let mime = match path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => "image/png",
        };

        Some(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
    } else {
        None
    };

    Ok(TemplateCtx { content, image })
}

/// 按字符数截断；超限时在结尾追加截断标记，让模型明确知道内容不完整。
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut out: String = text.chars().take(max_chars).collect();
    out.push_str(TRUNCATION_MARKER);

    out
}

/// 流式 chunk 的发射间隔：OCR 等密集输出每秒可产生数百个 delta，逐个 emit
/// IPC 事件会淹没问题前端（万级事件 → 界面卡死），按时间窗口攒批发射。
const CHUNK_EMIT_INTERVAL: Duration = Duration::from_millis(150);

/// 串联执行：渲染 → 组 messages → 按 streaming 开关流式/一次性请求 → emit。
/// 流式按窗口攒批 emit `ai://chunk`，两条路径结束都 emit `ai://done`。
pub async fn execute_ai_request(
    app: &tauri::AppHandle,
    streaming: bool,
    config: &ProviderConfig,
    action: &AiAction,
    ctx: &TemplateCtx,
    request_id: &str,
) -> Result<(), AiError> {
    use super::provider::{build_messages, chat_once, stream_chat};

    let started = std::time::Instant::now();
    let messages = build_messages(action, ctx)?;

    if !streaming {
        let full = chat_once(config, messages).await?;
        log::info!(
            "ai request {request_id} done (non-stream) in {:.1}s, {} chars",
            started.elapsed().as_secs_f32(),
            full.chars().count()
        );

        let _ = app.emit(
            events::AI_DONE,
            json!({
                "requestId": request_id,
                "text": full,
            }),
        );

        return Ok(());
    }

    let mut pending = String::new();
    let mut last_emit = Instant::now();
    let mut first_chunk_logged = false;
    let mut on_delta = |delta: String| {
        // 首字耗时 = 连接 + 中转/排队 + 模型首 token；与总时长对比即可区分
        // 「首字慢（排队/中转站）」还是「生成慢（输出长）」。
        if !first_chunk_logged {
            log::info!(
                "ai request {request_id} first chunk after {:.1}s",
                started.elapsed().as_secs_f32()
            );
            first_chunk_logged = true;
        }

        pending.push_str(&delta);

        if last_emit.elapsed() >= CHUNK_EMIT_INTERVAL {
            let _ = app.emit(
                events::AI_CHUNK,
                json!({
                    "requestId": request_id,
                    "delta": pending,
                }),
            );
            pending = String::new();
            last_emit = Instant::now();
        }
    };
    let full = stream_chat(config, messages, &mut on_delta).await?;

    if !pending.is_empty() {
        let _ = app.emit(
            events::AI_CHUNK,
            json!({
                "requestId": request_id,
                "delta": pending,
            }),
        );
    }

    log::info!(
        "ai request {request_id} done (stream) in {:.1}s, {} chars",
        started.elapsed().as_secs_f32(),
        full.chars().count()
    );

    let _ = app.emit(
        events::AI_DONE,
        json!({
            "requestId": request_id,
            "text": full,
        }),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ClipboardSubKind, Platform};

    #[test]
    fn rejects_empty_image_name() {
        assert!(is_unsafe_image_name(""));
    }

    #[test]
    fn rejects_forward_slash_in_image_name() {
        assert!(is_unsafe_image_name("foo/bar.png"));
    }

    #[test]
    fn rejects_backslash_in_image_name() {
        assert!(is_unsafe_image_name("foo\\bar.png"));
    }

    #[test]
    fn rejects_parent_dir_in_image_name() {
        assert!(is_unsafe_image_name("../bar.png"));
        assert!(is_unsafe_image_name("foo/.."));
    }

    #[test]
    fn accepts_plain_file_name() {
        assert!(!is_unsafe_image_name("abc123.png"));
        assert!(!is_unsafe_image_name("image-20260828.webp"));
    }

    fn is_unsafe_image_name(file_name: &str) -> bool {
        file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name.contains("..")
    }

    fn text_item(content: &str, search_text: Option<&str>) -> ClipboardItem {
        ClipboardItem {
            id: "item".to_owned(),
            kind: ClipboardKind::Text,
            sub_kind: Some(ClipboardSubKind::Html),
            group_id: None,
            source_app_id: None,
            content: content.to_owned(),
            content_hash: String::new(),
            search_text: search_text.map(str::to_owned),
            summary: None,
            file_types: None,
            size: None,
            width: None,
            height: None,
            use_count: 1,
            is_favorite: false,
            is_pinned: false,
            is_sensitive: false,
            platform: Platform::Windows,
            note: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            source_app_name: None,
            source_app_icon_file: None,
            source_app_icon_path: None,
            image_thumbnail_path: None,
            file_entries: None,
            files_preview_kind: None,
            available_actions: Vec::new(),
            color_preview: None,
            display_created_at: String::new(),
        }
    }

    fn store() -> (tempfile::TempDir, ImageStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ImageStore::for_test(dir.path().join("images"));

        (dir, store)
    }

    #[tokio::test]
    async fn text_ctx_prefers_search_text_plain() {
        // 网页复制的条目：content 存 HTML 源，search_text 存 OS 纯文本——
        // prompt 必须拿到后者（与「粘贴为纯文本」同源）。
        let item = text_item(
            "<html><body><!--StartFragment--><p>hello</p><!--EndFragment--></body></html>",
            Some("hello"),
        );

        let ctx = build_request_ctx(&item, &store().1).await.unwrap();

        assert_eq!(ctx.content.as_deref(), Some("hello"));
    }

    #[tokio::test]
    async fn text_ctx_falls_back_to_content_without_search_text() {
        let item = text_item("plain text", None);

        let ctx = build_request_ctx(&item, &store().1).await.unwrap();

        assert_eq!(ctx.content.as_deref(), Some("plain text"));
    }

    #[tokio::test]
    async fn text_ctx_truncates_with_marker() {
        let long = "字".repeat(MAX_TEXT_CHARS + 10);
        let item = text_item(&long, None);

        let ctx = build_request_ctx(&item, &store().1).await.unwrap();
        let content = ctx.content.unwrap();

        assert_eq!(
            content.chars().count(),
            MAX_TEXT_CHARS + TRUNCATION_MARKER.chars().count()
        );
        assert!(content.ends_with(TRUNCATION_MARKER.trim_start_matches('\n')));
    }

    #[tokio::test]
    async fn text_ctx_keeps_content_within_limit_untouched() {
        let item = text_item("短文本", None);

        let ctx = build_request_ctx(&item, &store().1).await.unwrap();

        assert_eq!(ctx.content.as_deref(), Some("短文本"));
    }

    fn image_item(file_name: &str) -> ClipboardItem {
        ClipboardItem {
            id: "img-item".to_owned(),
            kind: ClipboardKind::Image,
            sub_kind: None,
            group_id: None,
            source_app_id: None,
            content: file_name.to_owned(),
            content_hash: String::new(),
            search_text: None,
            summary: None,
            file_types: None,
            size: None,
            width: None,
            height: None,
            use_count: 1,
            is_favorite: false,
            is_pinned: false,
            is_sensitive: false,
            platform: Platform::Windows,
            note: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            source_app_name: None,
            source_app_icon_file: None,
            source_app_icon_path: None,
            image_thumbnail_path: None,
            file_entries: None,
            files_preview_kind: None,
            available_actions: Vec::new(),
            color_preview: None,
            display_created_at: String::new(),
        }
    }

    #[tokio::test]
    async fn image_ctx_returns_error_when_file_not_found() {
        let item = image_item("nonexistent.png");
        let (_, store) = store();

        let result = build_request_ctx(&item, &store).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn image_ctx_returns_error_when_file_too_large() {
        let (_dir, store) = store();
        let file_name = "big.png";
        let path = store.origin_path(file_name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, vec![0u8; MAX_IMAGE_BYTES + 1]).unwrap();

        let item = image_item(file_name);
        let result = build_request_ctx(&item, &store).await;

        assert!(result.is_err());
    }
}
