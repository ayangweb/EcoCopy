//! 列表项右键菜单（Rust 侧）：macOS 走原生 muda（[`clipboard_item::native`]），
//! Windows 走自定义 webview 窗（[`context_window`]，避免 muda `TrackPopupMenu`
//! 抢前台焦点）。

pub mod action;
pub mod clipboard_item;

#[cfg(target_os = "windows")]
pub mod context_window;
