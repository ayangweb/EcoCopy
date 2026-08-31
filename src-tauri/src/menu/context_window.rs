//! Windows 自定义右键菜单窗（webview）。
//!
//! 用 `focusable: false` 的 webview 窗替代 muda：muda 的 `TrackPopupMenu` 必须把
//! 菜单 owner 拉到前台，会把用户原本聚焦的目标 App（资源管理器重命名编辑框、
//! 浏览器地址栏 IME 等）挤掉焦点。本窗口不抢焦，跟剪贴板窗口一样隐形挂在桌面上。
//!
//! - 首次右键 / 悬停二级菜单时按需建窗；右键时 `set_size + set_position + emit + show`。
//! - 外部点击关闭：复用 [`crate::mouse`] 的全局鼠标钩子，菜单可见且光标在矩形外
//!   即 hide（不用轮询）。
//! - 菜单项点击：前端直接 emit `clipboard://menu-action` 给剪贴板窗口，业务派发与
//!   macOS 路径走同一套。

use std::sync::{LazyLock, Mutex};

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::commands::{
    ContextMenuItemPayload, ContextMenuShowPayload, ContextSubmenuGroupInput,
    ShowContextSubmenuInput,
};
use crate::core::{AppError, Result};
use crate::settings::Language;
use crate::window::lifecycle;

use super::clipboard_item::{
    visible_ordered_actions, AiMenuItem, ClipboardItemMenuRequest, ClipboardMenuAction,
    ClipboardMenuGroup, ACTION_GROUPS, AI_ACTION_PREFIX,
};

pub const CONTEXT_MENU_WINDOW_LABEL: &str = "context-menu";
pub const CONTEXT_SUBMENU_WINDOW_LABEL: &str = "context-submenu";

/// 前端订阅事件：菜单窗收到后渲染并 `show`。
const CONTEXT_MENU_SHOW_EVENT: &str = "context-menu://show";
const CONTEXT_SUBMENU_SHOW_EVENT: &str = "context-submenu://show";

// 几何常量（logical px）：跟前端 `ContextMenu` 的 CSS 必须一致，否则 hit-test
// 与裁切会错位。前端那侧用同名 token 写在 `ContextMenu/index.tsx` 头部。
const MENU_WIDTH: u32 = 220;
const SUBMENU_WIDTH: u32 = 220;
const ITEM_HEIGHT: u32 = 32;
const SEPARATOR_HEIGHT: u32 = 9;
const MENU_PADDING_Y: u32 = 8; // 上下各 4
const SURFACE_BORDER: u32 = 2;
const SUBMENU_GAP: u32 = 4;

static CONTEXT_BUILD_LOCK: Mutex<()> = Mutex::new(());
static CONTEXT_MENU_PAYLOAD: LazyLock<Mutex<Option<ContextMenuShowPayload>>> =
    LazyLock::new(|| Mutex::new(None));
static CONTEXT_SUBMENU_PAYLOAD: LazyLock<Mutex<Option<ShowContextSubmenuInput>>> =
    LazyLock::new(|| Mutex::new(None));

/// setup 阶段初始化 Windows 自定义右键菜单模块。
///
/// 菜单窗口改为按需创建，避免应用启动时加载隐藏 WebView。
pub fn init(app: &AppHandle) {
    let _ = app;
}

/// 按需重建一级右键菜单窗口。
pub fn build_context_menu_window(app: &AppHandle) -> Result<()> {
    build_menu_window(
        app,
        CONTEXT_MENU_WINDOW_LABEL,
        "index.html/#/context-menu",
        MENU_WIDTH,
        240,
    )?;

    Ok(())
}

/// 按需重建二级右键菜单窗口。
pub fn build_context_submenu_window(app: &AppHandle) -> Result<()> {
    build_menu_window(
        app,
        CONTEXT_SUBMENU_WINDOW_LABEL,
        "index.html/#/context-submenu",
        SUBMENU_WIDTH,
        120,
    )
}

fn build_menu_window(
    app: &AppHandle,
    label: &str,
    url: &str,
    width: u32,
    height: u32,
) -> Result<()> {
    let _guard = CONTEXT_BUILD_LOCK.lock().unwrap_or_else(|poisoned| {
        log::error!("context menu build mutex poisoned, recovering");
        poisoned.into_inner()
    });

    if app.get_webview_window(label).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .inner_size(width as f64, height as f64)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .focusable(false)
        .visible(false)
        .skip_taskbar(true)
        .drag_and_drop(false)
        .build()
        .map_err(|err| AppError::Other(anyhow::anyhow!("build {label} window: {err}")))?;

    Ok(())
}

/// 在当前光标处弹出列表项右键菜单。算好 size + position → `set_size`
/// → `set_position` → emit 数据 → `show`。
pub(super) fn show_for_clipboard_item(
    app: &AppHandle,
    request: &ClipboardItemMenuRequest,
) -> Result<()> {
    let lang = request.language;
    let menu_settings = &request.menu;

    let (promoted_ai, remaining_ai) =
        super::action::split_promoted_ai_actions(menu_settings, &request.ai_actions);

    let ctx = GroupsContext {
        clipboard_groups: &request.groups,
        current_group_id: request.current_group_id.as_deref(),
        lang,
        is_favorite: request.is_favorite,
        is_pinned: request.is_pinned,
        has_note: request.has_note,
        menu: menu_settings,
        promoted_ai: &promoted_ai,
        remaining_ai: &remaining_ai,
    };
    let groups = build_groups(&request.available_actions, &ctx);
    if groups.is_empty() {
        return Ok(());
    }

    let (width, height) = compute_size(&groups);
    let payload = ContextMenuShowPayload {
        item_id: request.item_id.clone(),
        is_favorite: request.is_favorite,
        is_pinned: request.is_pinned,
        groups,
    };
    set_context_menu_payload(Some(payload.clone()));

    hide_submenu(app);
    build_context_menu_window(app)?;

    let window = app
        .get_webview_window(CONTEXT_MENU_WINDOW_LABEL)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("context-menu window missing")))?;

    let (x, y) = compute_position(&window, width, height)?;

    window
        .set_size(LogicalSize::new(width as f64, height as f64))
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-menu set_size: {err}")))?;
    window
        .set_position(LogicalPosition::new(x as f64, y as f64))
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-menu set_position: {err}")))?;

    window
        .emit(CONTEXT_MENU_SHOW_EVENT, payload)
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-menu emit show: {err}")))?;
    window
        .show()
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-menu show: {err}")))?;
    lifecycle::on_shown(app, CONTEXT_MENU_WINDOW_LABEL);

    Ok(())
}

/// 根据一级菜单项矩形显示二级菜单窗口。
pub fn show_submenu(app: &AppHandle, input: ShowContextSubmenuInput) -> Result<()> {
    if input.groups.is_empty() {
        hide_submenu(app);

        return Ok(());
    }

    set_context_submenu_payload(Some(input.clone()));
    build_context_submenu_window(app)?;

    let root = app
        .get_webview_window(CONTEXT_MENU_WINDOW_LABEL)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("context-menu window missing")))?;
    let submenu = app
        .get_webview_window(CONTEXT_SUBMENU_WINDOW_LABEL)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("context-submenu window missing")))?;

    let (width, height) = compute_submenu_size(input.groups.len() as u32);
    let (x, y) = compute_submenu_position(&root, &input.anchor, width, height)?;

    submenu
        .set_size(LogicalSize::new(width as f64, height as f64))
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-submenu set_size: {err}")))?;
    submenu
        .set_position(LogicalPosition::new(x as f64, y as f64))
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-submenu set_position: {err}")))?;

    submenu
        .emit(CONTEXT_SUBMENU_SHOW_EVENT, input)
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-submenu emit show: {err}")))?;
    submenu
        .show()
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-submenu show: {err}")))?;
    lifecycle::on_shown(app, CONTEXT_SUBMENU_WINDOW_LABEL);

    Ok(())
}

fn action_group(action: ClipboardMenuAction) -> usize {
    for (i, group) in ACTION_GROUPS.iter().enumerate() {
        if group.contains(&action) {
            return i;
        }
    }

    ACTION_GROUPS.len()
}

struct GroupsContext<'a> {
    clipboard_groups: &'a [ClipboardMenuGroup],
    current_group_id: Option<&'a str>,
    lang: Language,
    is_favorite: bool,
    is_pinned: bool,
    has_note: bool,
    menu: &'a crate::settings::Menu,
    promoted_ai: &'a [AiMenuItem],
    remaining_ai: &'a [AiMenuItem],
}

fn build_groups(
    available: &[ClipboardMenuAction],
    ctx: &GroupsContext<'_>,
) -> Vec<Vec<ContextMenuItemPayload>> {
    let mut active = available.to_vec();
    if !ctx.clipboard_groups.is_empty() {
        active.push(ClipboardMenuAction::MoveToGroup);
    }

    let mut groups = Vec::new();
    let mut current_group: Vec<ContextMenuItemPayload> = Vec::new();
    let mut prev_group: Option<usize> = None;

    for action in visible_ordered_actions(ctx.menu) {
        if !active.contains(&action) {
            continue;
        }

        let group = action_group(action);

        if prev_group.is_some() && prev_group != Some(group) {
            groups.push(current_group);
            current_group = Vec::new();
        }
        prev_group = Some(group);

        current_group.push(ContextMenuItemPayload {
            action,
            label: action
                .label(ctx.lang, ctx.is_favorite, ctx.is_pinned, ctx.has_note)
                .into(),
            accelerator: action.accelerator().map(String::from),
            groups: build_group_items(action, ctx.clipboard_groups, ctx.current_group_id),
            ai_action_id: None,
        });
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    if !ctx.promoted_ai.is_empty() {
        groups.push(
            ctx.promoted_ai
                .iter()
                .map(|ai| ContextMenuItemPayload {
                    action: ClipboardMenuAction::AiProcess,
                    label: ai.label.clone(),
                    accelerator: None,
                    groups: Vec::new(),
                    ai_action_id: Some(ai.id.clone()),
                })
                .collect(),
        );
    }

    if !ctx.remaining_ai.is_empty() {
        groups.push(vec![ContextMenuItemPayload {
            action: ClipboardMenuAction::AiProcess,
            label: ClipboardMenuAction::AiProcess
                .label(ctx.lang, ctx.is_favorite, ctx.is_pinned, ctx.has_note)
                .into(),
            accelerator: None,
            groups: ctx
                .remaining_ai
                .iter()
                .map(|a| ContextSubmenuGroupInput {
                    checked: false,
                    id: format!("{}{}", AI_ACTION_PREFIX, a.id),
                    label: a.label.clone(),
                })
                .collect(),
            ai_action_id: None,
        }]);
    }

    groups
}

fn build_group_items(
    action: ClipboardMenuAction,
    clipboard_groups: &[ClipboardMenuGroup],
    current_group_id: Option<&str>,
) -> Vec<ContextSubmenuGroupInput> {
    if action != ClipboardMenuAction::MoveToGroup {
        return Vec::new();
    }

    clipboard_groups
        .iter()
        .map(|group| ContextSubmenuGroupInput {
            checked: current_group_id == Some(group.id.as_str()),
            id: group.id.clone(),
            label: group.name.clone(),
        })
        .collect()
}

fn compute_size(groups: &[Vec<ContextMenuItemPayload>]) -> (u32, u32) {
    let item_count: u32 = groups.iter().map(|g| g.len() as u32).sum();
    let separator_count = groups.len().saturating_sub(1) as u32;
    let root_height = compute_surface_height(item_count, separator_count);

    add_surface_border(MENU_WIDTH, root_height)
}

fn compute_submenu_size(item_count: u32) -> (u32, u32) {
    add_surface_border(SUBMENU_WIDTH, compute_surface_height(item_count, 0))
}

fn compute_surface_height(item_count: u32, separator_count: u32) -> u32 {
    MENU_PADDING_Y + item_count * ITEM_HEIGHT + separator_count * SEPARATOR_HEIGHT
}

fn add_surface_border(width: u32, height: u32) -> (u32, u32) {
    (width, height + SURFACE_BORDER)
}

/// 取光标所在显示器，把菜单矩形 clamp 在显示器内（鼠标处尽量为菜单左上角，
/// 右 / 下越界时翻到屏幕另一边）。返回 logical 坐标。
fn compute_position(window: &tauri::WebviewWindow, width: u32, height: u32) -> Result<(i32, i32)> {
    let cursor = window
        .cursor_position()
        .map_err(|err| AppError::Other(anyhow::anyhow!("cursor_position: {err}")))?;
    let scale = window
        .scale_factor()
        .map_err(|err| AppError::Other(anyhow::anyhow!("scale_factor: {err}")))?;
    let monitor = window
        .monitor_from_point(cursor.x, cursor.y)
        .map_err(|err| AppError::Other(anyhow::anyhow!("monitor_from_point: {err}")))?
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("no monitor under cursor")))?;

    let cx = (cursor.x / scale) as i32;
    let cy = (cursor.y / scale) as i32;
    let mon_x = (monitor.position().x as f64 / scale) as i32;
    let mon_y = (monitor.position().y as f64 / scale) as i32;
    let mon_w = (monitor.size().width as f64 / scale) as i32;
    let mon_h = (monitor.size().height as f64 / scale) as i32;

    let max_x = mon_x + mon_w - width as i32;
    let max_y = mon_y + mon_h - height as i32;

    // 右 / 下越界时翻到光标左 / 上侧，避免菜单贴边裁切。
    let x = if cx > max_x { cx - width as i32 } else { cx }.clamp(mon_x, max_x.max(mon_x));
    let y = if cy > max_y { cy - height as i32 } else { cy }.clamp(mon_y, max_y.max(mon_y));

    Ok((x, y))
}

fn compute_submenu_position(
    root: &tauri::WebviewWindow,
    anchor: &crate::commands::ContextSubmenuAnchor,
    width: u32,
    height: u32,
) -> Result<(i32, i32)> {
    let scale = root
        .scale_factor()
        .map_err(|err| AppError::Other(anyhow::anyhow!("scale_factor: {err}")))?;
    let root_position = root
        .outer_position()
        .map_err(|err| AppError::Other(anyhow::anyhow!("context-menu outer_position: {err}")))?;
    let probe_x = root_position.x + ((anchor.left + anchor.width / 2.0) * scale) as i32;
    let probe_y = root_position.y + ((anchor.top + anchor.height / 2.0) * scale) as i32;
    let monitor = root
        .monitor_from_point(probe_x as f64, probe_y as f64)
        .map_err(|err| AppError::Other(anyhow::anyhow!("monitor_from_point: {err}")))?
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("no monitor under context menu")))?;

    let root_rect = LogicalRect {
        x: root_position.x as f64 / scale,
        y: root_position.y as f64 / scale,
        width: 0.0,
        height: 0.0,
    };
    let monitor_rect = LogicalRect {
        x: monitor.position().x as f64 / scale,
        y: monitor.position().y as f64 / scale,
        width: monitor.size().width as f64 / scale,
        height: monitor.size().height as f64 / scale,
    };

    Ok(compute_submenu_position_in_monitor(
        root_rect,
        LogicalRect {
            x: anchor.left,
            y: anchor.top,
            width: anchor.width,
            height: anchor.height,
        },
        monitor_rect,
        width,
        height,
    ))
}

#[derive(Debug, Clone, Copy)]
struct LogicalRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn compute_submenu_position_in_monitor(
    root: LogicalRect,
    anchor: LogicalRect,
    monitor: LogicalRect,
    width: u32,
    height: u32,
) -> (i32, i32) {
    let width = width as f64;
    let height = height as f64;
    let gap = SUBMENU_GAP as f64;
    let monitor_right = monitor.x + monitor.width;
    let monitor_bottom = monitor.y + monitor.height;
    let max_x = (monitor_right - width).max(monitor.x);
    let max_y = (monitor_bottom - height).max(monitor.y);

    let right_surface_x = root.x + anchor.x + anchor.width + gap;
    let left_surface_x = root.x + anchor.x - gap - SUBMENU_WIDTH as f64;
    let preferred_x = if right_surface_x + width <= monitor_right {
        right_surface_x
    } else {
        left_surface_x
    };
    let y = root.y + anchor.y;

    (
        preferred_x.clamp(monitor.x, max_x) as i32,
        y.clamp(monitor.y, max_y) as i32,
    )
}

/// 返回最近一次一级右键菜单 show 请求的 payload，供重建后的 WebView 首屏补拉。
pub fn context_menu_payload() -> Option<ContextMenuShowPayload> {
    CONTEXT_MENU_PAYLOAD
        .lock()
        .unwrap_or_else(|poisoned| {
            log::error!("context menu payload mutex poisoned, recovering");
            poisoned.into_inner()
        })
        .clone()
}

/// 返回最近一次二级右键菜单 show 请求的 payload，供重建后的 WebView 首屏补拉。
pub fn context_submenu_payload() -> Option<ShowContextSubmenuInput> {
    CONTEXT_SUBMENU_PAYLOAD
        .lock()
        .unwrap_or_else(|poisoned| {
            log::error!("context submenu payload mutex poisoned, recovering");
            poisoned.into_inner()
        })
        .clone()
}

/// 更新一级右键菜单首屏补拉 payload。
fn set_context_menu_payload(payload: Option<ContextMenuShowPayload>) {
    let mut guard = CONTEXT_MENU_PAYLOAD.lock().unwrap_or_else(|poisoned| {
        log::error!("context menu payload mutex poisoned on set, recovering");
        poisoned.into_inner()
    });
    *guard = payload;
}

/// 更新二级右键菜单首屏补拉 payload。
fn set_context_submenu_payload(payload: Option<ShowContextSubmenuInput>) {
    let mut guard = CONTEXT_SUBMENU_PAYLOAD.lock().unwrap_or_else(|poisoned| {
        log::error!("context submenu payload mutex poisoned on set, recovering");
        poisoned.into_inner()
    });
    *guard = payload;
}

/// 隐藏菜单窗。给鼠标钩子 / 剪贴板窗口隐藏等外部触发处用。
pub fn hide(app: &AppHandle) {
    set_context_menu_payload(None);
    set_context_submenu_payload(None);

    for label in [CONTEXT_SUBMENU_WINDOW_LABEL, CONTEXT_MENU_WINDOW_LABEL] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        if let Err(err) = window.hide() {
            log::warn!("hide {label} window failed: {err}");
            continue;
        }
        lifecycle::on_hidden(app, label, "context-menu-hide");
    }
}

/// 隐藏二级菜单窗。
pub fn hide_submenu(app: &AppHandle) {
    set_context_submenu_payload(None);

    let Some(window) = app.get_webview_window(CONTEXT_SUBMENU_WINDOW_LABEL) else {
        return;
    };
    if let Err(err) = window.hide() {
        log::warn!("hide context-submenu window failed: {err}");
        return;
    }
    lifecycle::on_hidden(app, CONTEXT_SUBMENU_WINDOW_LABEL, "context-submenu-hide");
}

/// 菜单窗当前是否可见。给鼠标钩子判断「是否需要做外部点击关闭」。
pub fn is_visible(app: &AppHandle) -> bool {
    [CONTEXT_MENU_WINDOW_LABEL, CONTEXT_SUBMENU_WINDOW_LABEL]
        .iter()
        .any(|label| {
            app.get_webview_window(label)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false)
        })
}

/// 判断 physical 坐标是否落在任一菜单窗口矩形内。
pub fn contains_physical_point(app: &AppHandle, x: i32, y: i32) -> bool {
    [CONTEXT_MENU_WINDOW_LABEL, CONTEXT_SUBMENU_WINDOW_LABEL]
        .iter()
        .any(|label| {
            let Some(window) = app.get_webview_window(label) else {
                return false;
            };
            if !window.is_visible().unwrap_or(false) {
                return false;
            }
            let Ok(position) = window.outer_position() else {
                return false;
            };
            let Ok(size) = window.outer_size() else {
                return false;
            };

            x >= position.x
                && x < position.x + size.width as i32
                && y >= position.y
                && y < position.y + size.height as i32
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn compute_size_keeps_root_menu_single_column() {
        let groups = vec![
            vec![ContextMenuItemPayload {
                action: ClipboardMenuAction::Paste,
                label: "Paste".to_owned(),
                accelerator: None,
                groups: Vec::new(),
                ai_action_id: None,
            }],
            vec![ContextMenuItemPayload {
                action: ClipboardMenuAction::MoveToGroup,
                label: "Move".to_owned(),
                accelerator: None,
                groups: vec![ContextSubmenuGroupInput {
                    checked: false,
                    id: "g1".to_owned(),
                    label: "Group".to_owned(),
                }],
                ai_action_id: None,
            }],
        ];

        assert_eq!(
            compute_size(&groups),
            (
                MENU_WIDTH,
                MENU_PADDING_Y + ITEM_HEIGHT * 2 + SEPARATOR_HEIGHT + SURFACE_BORDER,
            )
        );
    }

    #[test]
    fn compute_submenu_position_prefers_right_side() {
        let position = compute_submenu_position_in_monitor(
            rect(100.0, 100.0, 0.0, 0.0),
            rect(8.0, 72.0, 220.0, 32.0),
            rect(0.0, 0.0, 800.0, 600.0),
            SUBMENU_WIDTH,
            120,
        );

        assert_eq!(position, (332, 172));
    }

    #[test]
    fn compute_submenu_position_flips_left_when_right_overflows() {
        let position = compute_submenu_position_in_monitor(
            rect(560.0, 100.0, 0.0, 0.0),
            rect(8.0, 72.0, 220.0, 32.0),
            rect(0.0, 0.0, 800.0, 600.0),
            SUBMENU_WIDTH,
            120,
        );

        assert_eq!(position, (344, 172));
    }

    #[test]
    fn compute_submenu_position_clamps_to_monitor_bottom() {
        let position = compute_submenu_position_in_monitor(
            rect(100.0, 520.0, 0.0, 0.0),
            rect(8.0, 72.0, 220.0, 32.0),
            rect(0.0, 0.0, 800.0, 600.0),
            SUBMENU_WIDTH,
            120,
        );

        assert_eq!(position, (332, 480));
    }

    #[test]
    fn compute_submenu_size_returns_visible_surface_window() {
        assert_eq!(
            compute_submenu_size(2),
            (
                SUBMENU_WIDTH,
                MENU_PADDING_Y + ITEM_HEIGHT * 2 + SURFACE_BORDER,
            )
        );
    }
}
