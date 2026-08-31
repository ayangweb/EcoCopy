//! Command wrappers for the Windows custom context-menu webviews.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::core::Result;
use crate::menu::clipboard_item::ClipboardMenuAction;

/// Single group row rendered by the submenu window.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSubmenuGroupInput {
    pub checked: bool,
    pub id: String,
    pub label: String,
}

/// Single row rendered by the root context menu window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuItemPayload {
    pub action: ClipboardMenuAction,
    pub label: String,
    pub accelerator: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ContextSubmenuGroupInput>,
    pub ai_action_id: Option<String>,
}

/// Full payload needed to render the root context menu window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuShowPayload {
    pub item_id: String,
    pub is_favorite: bool,
    pub is_pinned: bool,
    /// 已按后端动作分组过滤排序；前端按二维结构渲染并自动插入分隔符。
    pub groups: Vec<Vec<ContextMenuItemPayload>>,
}

/// Anchor rectangle relative to the root menu webview viewport.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSubmenuAnchor {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// Full payload needed to show the submenu window.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowContextSubmenuInput {
    pub action: ClipboardMenuAction,
    pub anchor: ContextSubmenuAnchor,
    pub groups: Vec<ContextSubmenuGroupInput>,
    pub item_id: String,
}

/// Returns the latest pending root context-menu payload for a freshly rebuilt window.
#[tauri::command]
pub async fn get_context_menu_payload() -> Result<Option<ContextMenuShowPayload>> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::menu::context_window::context_menu_payload())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

/// Returns the latest pending submenu payload for a freshly rebuilt window.
#[tauri::command]
pub async fn get_context_submenu_payload() -> Result<Option<ShowContextSubmenuInput>> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::menu::context_window::context_submenu_payload())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

/// Shows the secondary Windows custom context-menu window.
#[tauri::command]
pub async fn show_context_submenu(app: AppHandle, input: ShowContextSubmenuInput) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        crate::menu::context_window::show_submenu(&app, input)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        let _ = input;

        Ok(())
    }
}

/// Hides the secondary Windows custom context-menu window.
#[tauri::command]
pub async fn hide_context_submenu(app: AppHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        crate::menu::context_window::hide_submenu(&app);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
    }

    Ok(())
}

/// Hides both Windows custom context-menu windows.
#[tauri::command]
pub async fn hide_context_menus(app: AppHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        crate::menu::context_window::hide(&app);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
    }

    Ok(())
}
