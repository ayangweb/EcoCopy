//! 剪贴板写回：把 [`ClipboardItem`] 按类型写回系统剪贴板（text / html / rtf / image / files）。
//!
//! 时序约束：[`ClipboardContext`] 是 `!Send`，调用方需在不跨 await 的同步段内完成调用
//! （命令层照 `read_clipboard` 的写法处理）。
//!
//! 回环抑制：写回前向 [`WritebackGuard`] 登记将写入内容的 `content_hash`，
//! OS 监听重新读到同内容时跳过入库，避免「点击粘贴 → 自动新增一条」回环。
//! 哈希必须与 [`crate::clipboard::ingest::build_item`] 在 watcher 路径上将算出的哈希一致：
//! - text / html / rtf：watcher 拿到的 plain/html/rtf 经 `draft_from_text` 后 `content` 即我们写入的串，
//!   `content_hash(Text, written)` 自然匹配；
//! - files：watcher 把路径列表用 `\n` 连接后哈希，与我们 `item.content` 一致；
//! - image：watcher 把 PNG 字节再 sha256 → 文件名 → 哈希。前提是 OS pasteboard 不改像素，
//!   且 clipboard-rs 的 PNG 重新编码确定。绝大多数复制路径满足，极端情况可能漏抑制一次（最多多入一条新行）。
//!
//! 纯文本模式（`plain = true`）：忽略 `sub_kind`，写 `search_text`（OS 提供的纯文本表示），
//! 缺失时退回 `content`。供「纯文本粘贴」快捷路径使用。

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext, RustImageData};

use super::guard::WritebackGuard;
use super::storage::ImageStore;
use crate::core::{AppError, Result};
use crate::db::items::content_hash;
use crate::db::models::{ClipboardItem, ClipboardKind, ClipboardSubKind};

/// 把 `item` 写回系统剪贴板；`plain = true` 强制只写纯文本（剥离 HTML/RTF）。
pub fn write_to_clipboard(
    store: &ImageStore,
    guard: &WritebackGuard,
    item: &ClipboardItem,
    plain: bool,
) -> Result<()> {
    let ctx = ClipboardContext::new().map_err(clip_err)?;

    match item.kind {
        ClipboardKind::Text => write_text(&ctx, guard, item, plain)?,
        ClipboardKind::Image => write_image(&ctx, store, guard, item)?,
        // files + plain：把路径列表当文本写回，供「粘贴为路径」使用。
        ClipboardKind::Files if plain => write_files_as_text(&ctx, guard, item)?,
        ClipboardKind::Files => write_files(&ctx, guard, item)?,
    }
    Ok(())
}

fn write_text(
    ctx: &ClipboardContext,
    guard: &WritebackGuard,
    item: &ClipboardItem,
    plain: bool,
) -> Result<()> {
    // 纯文本模式下，OS 提供的 plain 表示优先；缺失时退回 content（plain 文本场景下 content 即纯文本）。
    let (content, sub_kind) = if plain {
        let text = item
            .search_text
            .clone()
            .unwrap_or_else(|| item.content.clone());
        (text, None)
    } else {
        (item.content.clone(), item.sub_kind)
    };

    guard.suppress(content_hash(ClipboardKind::Text, &content));

    match sub_kind {
        // 纯文本模式必须只写 Text flavor，确保清掉剪贴板中可能残留的 HTML/RTF。
        None if plain => ctx
            .set(vec![ClipboardContent::Text(content)])
            .map_err(clip_err)?,
        // HTML / RTF 必须同时写入纯文本回退：clipboard-rs 的 set_html / set_rich_text
        // 会先 clearContents，单独写时只剩富格式，多数应用读 plain/text 拿不到就拒绝粘贴。
        // 走 set(Vec<ClipboardContent>) 一次写多格式（内部不再相互清空）。
        Some(ClipboardSubKind::Html) => {
            let plain = item.search_text.clone().unwrap_or_else(|| content.clone());
            guard.suppress(content_hash(ClipboardKind::Text, &plain));
            ctx.set(vec![
                ClipboardContent::Text(plain),
                ClipboardContent::Html(content),
            ])
            .map_err(clip_err)?;
        }
        Some(ClipboardSubKind::Rtf) => {
            let plain = item.search_text.clone().unwrap_or_else(|| content.clone());
            guard.suppress(content_hash(ClipboardKind::Text, &plain));
            ctx.set(vec![
                ClipboardContent::Text(plain),
                ClipboardContent::Rtf(content),
            ])
            .map_err(clip_err)?;
        }
        // url / email / color / path 及无 sub_kind 都走纯文本通道。
        _ => ctx.set_text(content).map_err(clip_err)?,
    }
    Ok(())
}

fn write_image(
    ctx: &ClipboardContext,
    store: &ImageStore,
    guard: &WritebackGuard,
    item: &ClipboardItem,
) -> Result<()> {
    let path = store.origin_path(&item.content);
    let bytes = std::fs::read(&path).map_err(|err| {
        log::error!("read image {path:?} failed: {err}");
        AppError::Clipboard(err.to_string())
    })?;
    let image = RustImageData::from_bytes(&bytes).map_err(clip_err)?;

    guard.suppress(item.content_hash.clone());
    ctx.set_image(image).map_err(clip_err)?;
    Ok(())
}

fn write_files(ctx: &ClipboardContext, guard: &WritebackGuard, item: &ClipboardItem) -> Result<()> {
    let paths: Vec<String> = item
        .content
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if paths.is_empty() {
        return Err(AppError::Clipboard("no files to write".to_owned()));
    }

    guard.suppress(item.content_hash.clone());
    ctx.set_files(paths).map_err(clip_err)?;
    Ok(())
}

/// 把 files 条目的路径列表当文本写回（换行分隔，多文件按行展开）。
/// 与 `write_files` 共用 `content_hash` 抑制——OS 监听不会拿到与原文本完全一致的回环。
fn write_files_as_text(
    ctx: &ClipboardContext,
    guard: &WritebackGuard,
    item: &ClipboardItem,
) -> Result<()> {
    let text = item.content.clone();

    guard.suppress(content_hash(ClipboardKind::Text, &text));
    ctx.set_text(text).map_err(clip_err)?;
    Ok(())
}

fn clip_err<E: std::fmt::Display>(err: E) -> AppError {
    AppError::Clipboard(err.to_string())
}

/// 把 AI 生成的文本写回系统剪贴板（不依赖 DB 记录）。
///
/// 与 `write_to_clipboard` 的区别：入参是裸文本而非 `ClipboardItem`，因为 AI
/// 结果尚未入库。写回前 trim 后登记 `content_hash`（与监听侧 `draft_from_text`
/// 同源），写后回读校验是否真落到剪贴板。
pub fn write_ai_text_to_clipboard(guard: &WritebackGuard, text: &str) -> Result<()> {
    // 监听侧 `draft_from_text` 会对读回内容先 trim 再算哈希，登记指纹必须同源
    // （模型输出常带首尾空白），否则回环抑制失效、结果被当成新复制入库。
    let trimmed = text.trim();
    guard.suppress(content_hash(ClipboardKind::Text, trimmed));

    let ctx = ClipboardContext::new().map_err(clip_err)?;
    ctx.set_text(trimmed.to_owned()).map_err(clip_err)?;

    // 写后回读校验：clipboard-rs 返回 Ok 不保证内容真落到系统剪贴板（可能被
    // 其它占用剪贴板的程序吞掉），读回不一致记 error 供排查「提示已写回但没写上」。
    let read_back = ctx.get_text().map_err(clip_err)?;
    if read_back.trim() != trimmed {
        log::error!(
            "ai writeback read-back mismatch: wrote {} chars, read back {} chars",
            trimmed.chars().count(),
            read_back.chars().count()
        );
        return Err(AppError::Ai("写回剪贴板未生效，请重试".into()));
    }

    log::info!("ai writeback ok: {} chars", trimmed.chars().count());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::payload::ImagePayload;
    use super::super::read::ClipboardReader;
    use super::*;
    use crate::clipboard::{build_item, ImageStore, WritebackGuard};
    use crate::db::models::Platform;
    use chrono::Utc;

    fn text_item(
        content: &str,
        sub: Option<ClipboardSubKind>,
        search: Option<&str>,
    ) -> ClipboardItem {
        ClipboardItem {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ClipboardKind::Text,
            sub_kind: sub,
            group_id: None,
            source_app_id: None,
            content_hash: content_hash(ClipboardKind::Text, content),
            content: content.to_owned(),
            search_text: search.map(str::to_owned),
            summary: None,
            file_types: None,
            size: None,
            width: None,
            height: None,
            use_count: 1,
            is_favorite: false,
            is_pinned: false,
            is_sensitive: false,
            platform: Platform::Macos,
            note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
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

    fn temp_store() -> (TempDir, ImageStore) {
        let dir = TempDir::new();
        let store = ImageStore::for_test(dir.path().join("resources").join("clipboard-images"));
        (dir, store)
    }

    // 触碰真实剪贴板：写入纯文本 → 读回应为同串，且 guard 已登记本次哈希。
    #[test]
    #[ignore = "touches the real system clipboard; run with --ignored on a desktop session"]
    fn writes_plain_text_and_arms_guard() {
        let _serial = crate::clipboard::test_lock::serial();
        let (_dir, store) = temp_store();
        let guard = WritebackGuard::new();

        let item = text_item("hello write", None, None);
        write_to_clipboard(&store, &guard, &item, false).unwrap();

        let reader = ClipboardReader::new().unwrap();
        let payload = reader
            .read_with_capture(&crate::settings::Capture::default())
            .unwrap()
            .expect("should read");
        let read_item = build_item(&store, &payload).unwrap().unwrap();
        assert_eq!(read_item.content, "hello write");
        assert!(guard.should_skip(&read_item.content_hash));
    }

    // 纯文本模式：强制丢弃 HTML，写 search_text。
    #[test]
    #[ignore = "touches the real system clipboard; run with --ignored on a desktop session"]
    fn plain_mode_strips_html() {
        let _serial = crate::clipboard::test_lock::serial();
        let (_dir, store) = temp_store();
        let guard = WritebackGuard::new();

        let item = text_item(
            "<b>Hello</b> World",
            Some(ClipboardSubKind::Html),
            Some("Hello World"),
        );
        write_to_clipboard(&store, &guard, &item, true).unwrap();

        let reader = ClipboardReader::new().unwrap();
        let payload = reader
            .read_with_capture(&crate::settings::Capture::default())
            .unwrap()
            .expect("should read");
        let read_item = build_item(&store, &payload).unwrap().unwrap();
        assert_eq!(read_item.kind, ClipboardKind::Text);
        assert_eq!(read_item.sub_kind, None);
        assert_eq!(read_item.content, "Hello World");
    }

    // 图片往返：写盘上的 PNG → 写剪贴板 → 读回 → 落盘的文件名应一致（去重哈希命中）。
    #[test]
    #[ignore = "touches the real system clipboard; run with --ignored on a desktop session"]
    fn round_trip_image_matches_hash() {
        let _serial = crate::clipboard::test_lock::serial();
        let (_dir, store) = temp_store();
        let guard = WritebackGuard::new();

        // 先落盘一张原图（模拟历史记录里的 image item）。
        let png = sample_png(48, 32);
        let stored = store
            .store(&ImagePayload {
                bytes: png,
                width: 48,
                height: 32,
            })
            .unwrap();
        let item = ClipboardItem {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ClipboardKind::Image,
            sub_kind: None,
            group_id: None,
            source_app_id: None,
            content_hash: content_hash(ClipboardKind::Image, &stored.file_name),
            content: stored.file_name.clone(),
            search_text: None,
            summary: None,
            file_types: None,
            size: Some(stored.size),
            width: Some(stored.width),
            height: Some(stored.height),
            use_count: 1,
            is_favorite: false,
            is_pinned: false,
            is_sensitive: false,
            platform: Platform::Macos,
            note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_app_name: None,
            source_app_icon_file: None,
            source_app_icon_path: None,
            image_thumbnail_path: None,
            file_entries: None,
            files_preview_kind: None,
            available_actions: Vec::new(),
            color_preview: None,
            display_created_at: String::new(),
        };

        write_to_clipboard(&store, &guard, &item, false).unwrap();

        let reader = ClipboardReader::new().unwrap();
        let payload = reader
            .read_with_capture(&crate::settings::Capture::default())
            .unwrap()
            .expect("should read image");
        let read_item = build_item(&store, &payload).unwrap().unwrap();
        assert_eq!(read_item.kind, ClipboardKind::Image);
        // 往返期望 PNG 字节哈希一致 → 同 content_hash → guard 抑制。
        assert_eq!(read_item.content_hash, item.content_hash);
        assert!(guard.should_skip(&read_item.content_hash));
    }

    fn sample_png(w: u32, h: u32) -> Vec<u8> {
        use std::io::Cursor;
        let buf = image::RgbaImage::from_pixel(w, h, image::Rgba([4, 5, 6, 255]));
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(buf)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    struct TempDir(std::path::PathBuf);
    impl TempDir {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("ecopaste-write-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }
}
