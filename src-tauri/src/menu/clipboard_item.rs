//! 列表项右键菜单（Rust 侧）。
//!
//! - **macOS**：用原生 muda `Menu` + `popup_menu`。NSPanel 不 resignKey，菜单
//!   弹出不会偷走前台焦点；菜单实例由 [`native::ClipboardItemMenuState`] 持有
//!   到下次 popup 前替换，规避 tauri-apps/tauri#9470 的 muda use-after-free。
//! - **Windows**：muda 的 `TrackPopupMenu` 必须把菜单 owner 拉到前台，会把用户
//!   原本聚焦的目标 App（如资源管理器重命名编辑框）挤掉焦点。改用自定义
//!   webview 窗（`focusable: false`，不偷焦点）实现，逻辑在
//!   [`super::context_window`]。
//!
//! 业务侧（toast / 二次确认 modal / 列表本地镜像同步）仍在前端 `List.tsx`
//! 维护，本模块只负责「弹菜单 + 点击后 emit `clipboard://menu-action` 给前端」。

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::core::Result;
use crate::db::DatabaseState;
use crate::settings::Language;

/// 前端订阅事件：携带 `{action, itemId}`，由 `List.tsx` 派发到现有处理逻辑。
#[cfg(target_os = "macos")]
pub const CLIPBOARD_MENU_ACTION_EVENT: &str = "clipboard://menu-action";

/// 与前端 `ClipboardAction` 对齐（`serde(rename_all = "camelCase")`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardMenuAction {
    Paste,
    PasteAsPlainText,
    PasteAsPath,
    Copy,
    SaveImage,
    OpenLink,
    SendEmail,
    RevealInFinder,
    RevealInExplorer,
    ToggleFavorite,
    TogglePinned,
    MoveToGroup,
    EditNote,
    Delete,
}

impl ClipboardMenuAction {
    /// 返回当前语言下的菜单文案；切换类动作按当前状态翻转。
    pub(super) fn label(
        self,
        lang: Language,
        is_favorite: bool,
        is_pinned: bool,
        has_note: bool,
    ) -> &'static str {
        use crate::i18n::clipboard_menu::Key;

        let key = match self {
            Self::Paste => Key::Paste,
            Self::PasteAsPlainText => Key::PasteAsPlainText,
            Self::PasteAsPath => Key::PasteAsPath,
            Self::Copy => Key::Copy,
            Self::SaveImage => Key::SaveImage,
            Self::OpenLink => Key::OpenLink,
            Self::SendEmail => Key::SendEmail,
            Self::RevealInFinder => Key::RevealInFinder,
            Self::RevealInExplorer => Key::RevealInExplorer,
            Self::ToggleFavorite => {
                if is_favorite {
                    Key::Unfavorite
                } else {
                    Key::Favorite
                }
            }
            Self::TogglePinned => {
                if is_pinned {
                    Key::UnpinItem
                } else {
                    Key::PinItem
                }
            }
            Self::MoveToGroup => Key::MoveToGroup,
            Self::EditNote => {
                if has_note {
                    Key::EditNote
                } else {
                    Key::AddNote
                }
            }
            Self::Delete => Key::Delete,
        };

        crate::i18n::clipboard_menu::label(lang, key)
    }

    /// 加速键文案（muda 与前端菜单共用 `"CmdOrCtrl+X"` 平台无关写法）。
    pub(super) fn accelerator(self) -> Option<&'static str> {
        match self {
            Self::Paste => Some("Enter"),
            Self::PasteAsPlainText | Self::PasteAsPath => Some("CmdOrCtrl+Enter"),
            Self::Copy => Some("CmdOrCtrl+C"),
            Self::SaveImage => None,
            Self::OpenLink | Self::SendEmail | Self::RevealInFinder | Self::RevealInExplorer => {
                Some("CmdOrCtrl+O")
            }
            Self::ToggleFavorite => Some("CmdOrCtrl+D"),
            Self::TogglePinned => Some("CmdOrCtrl+T"),
            Self::MoveToGroup => None,
            Self::EditNote => Some("CmdOrCtrl+M"),
            Self::Delete => Some("CmdOrCtrl+Backspace"),
        }
    }
}

/// 视觉分组：组间插入分隔线，组内顺序与组顺序即菜单展示顺序。两端共用。
pub const ACTION_GROUPS: &[&[ClipboardMenuAction]] = &[
    &[
        ClipboardMenuAction::Paste,
        ClipboardMenuAction::PasteAsPlainText,
        ClipboardMenuAction::PasteAsPath,
        ClipboardMenuAction::Copy,
        ClipboardMenuAction::SaveImage,
    ],
    &[
        ClipboardMenuAction::OpenLink,
        ClipboardMenuAction::SendEmail,
        ClipboardMenuAction::RevealInFinder,
        ClipboardMenuAction::RevealInExplorer,
    ],
    &[
        ClipboardMenuAction::ToggleFavorite,
        ClipboardMenuAction::TogglePinned,
        ClipboardMenuAction::MoveToGroup,
        ClipboardMenuAction::EditNote,
    ],
    &[ClipboardMenuAction::Delete],
];

/// 按 `menu.order` 的顺序过滤 `menu.visible_actions`，返回实际展示的动作列表。
pub fn visible_ordered_actions(menu: &crate::settings::Menu) -> Vec<ClipboardMenuAction> {
    menu.order
        .iter()
        .copied()
        .filter(|a| menu.visible_actions.contains(a))
        .collect()
}

/// 右键菜单里的可选自定义分组；由命令入口从数据库实时读取。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClipboardMenuGroup {
    pub id: String,
    pub name: String,
}

/// 前端弹出右键菜单命令的入参。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupClipboardItemMenuInput {
    pub item_id: String,
    pub available_actions: Vec<ClipboardMenuAction>,
    pub current_group_id: Option<String>,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub has_note: bool,
}

/// 构建右键菜单所需的完整上下文，包含命令参数与实时读取的分组列表。
#[derive(Debug, Clone)]
pub(super) struct ClipboardItemMenuRequest {
    pub item_id: String,
    pub available_actions: Vec<ClipboardMenuAction>,
    pub groups: Vec<ClipboardMenuGroup>,
    pub current_group_id: Option<String>,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub has_note: bool,
}

/// 菜单点击后 emit 给前端的 payload。Windows 自定义菜单窗也复用这个结构发回
/// 剪贴板窗口，前端 `List.tsx` 只需订阅一次。
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MenuActionPayload {
    pub action: ClipboardMenuAction,
    pub item_id: String,
    pub group_id: Option<String>,
}

// ============================================================================
// macOS：muda 原生菜单
// ============================================================================

#[cfg(target_os = "macos")]
mod native {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use anyhow::Context;
    use tauri::menu::{
        CheckMenuItem, IsMenuItem, Menu, MenuBuilder, MenuItem, PredefinedMenuItem, Submenu,
    };
    use tauri::{AppHandle, Emitter, Manager, Wry};

    use crate::core::{AppError, Result};
    use crate::settings::Language;
    use crate::window::CLIPBOARD_WINDOW_LABEL;

    use super::{
        ClipboardItemMenuRequest, ClipboardMenuAction, ClipboardMenuGroup, MenuActionPayload,
        ACTION_GROUPS, CLIPBOARD_MENU_ACTION_EVENT,
    };

    /// 菜单项 id 前缀；`on_menu_event` 按前缀分流到本模块，避免与托盘菜单 id 冲突。
    const MENU_PREFIX: &str = "cim::";
    const MOVE_GROUP_PREFIX: &str = "cim::moveToGroup::";

    impl ClipboardMenuAction {
        fn id(self) -> &'static str {
            match self {
                Self::Paste => "cim::paste",
                Self::PasteAsPlainText => "cim::pasteAsPlainText",
                Self::PasteAsPath => "cim::pasteAsPath",
                Self::Copy => "cim::copy",
                Self::SaveImage => "cim::saveImage",
                Self::OpenLink => "cim::openLink",
                Self::SendEmail => "cim::sendEmail",
                Self::RevealInFinder => "cim::revealInFinder",
                Self::RevealInExplorer => "cim::revealInExplorer",
                Self::ToggleFavorite => "cim::toggleFavorite",
                Self::TogglePinned => "cim::togglePinned",
                Self::MoveToGroup => "cim::moveToGroup",
                Self::EditNote => "cim::editNote",
                Self::Delete => "cim::delete",
            }
        }

        fn from_id(id: &str) -> Option<Self> {
            const ALL: &[ClipboardMenuAction] = &[
                ClipboardMenuAction::Paste,
                ClipboardMenuAction::PasteAsPlainText,
                ClipboardMenuAction::PasteAsPath,
                ClipboardMenuAction::Copy,
                ClipboardMenuAction::SaveImage,
                ClipboardMenuAction::OpenLink,
                ClipboardMenuAction::SendEmail,
                ClipboardMenuAction::RevealInFinder,
                ClipboardMenuAction::RevealInExplorer,
                ClipboardMenuAction::ToggleFavorite,
                ClipboardMenuAction::TogglePinned,
                ClipboardMenuAction::MoveToGroup,
                ClipboardMenuAction::EditNote,
                ClipboardMenuAction::Delete,
            ];
            if id.starts_with(MOVE_GROUP_PREFIX) {
                return Some(ClipboardMenuAction::MoveToGroup);
            }

            ALL.iter().copied().find(|a| a.id() == id)
        }
    }

    fn move_group_id(group_id: &str) -> String {
        format!("{MOVE_GROUP_PREFIX}{group_id}")
    }

    fn group_id_from_menu_id(menu_id: &str) -> Option<String> {
        menu_id.strip_prefix(MOVE_GROUP_PREFIX).map(str::to_owned)
    }

    /// 当前活跃的菜单 + 正在弹菜单的目标项 id；菜单持有到下次 popup 前替换，
    /// 保证事件派发期间不会 use-after-free。
    #[derive(Default)]
    pub(super) struct ClipboardItemMenuState {
        current: Mutex<Option<Menu<Wry>>>,
        target_item_id: Mutex<Option<String>>,
    }

    pub(super) fn init(app: &AppHandle) {
        app.manage(ClipboardItemMenuState::default());
    }

    /// 命令本身**立刻返回**：菜单的构建与弹出都被丢到主线程上异步执行。
    /// muda 的 `MenuItem::with_id` 与 `popup_menu` 都必须主线程，且
    /// `popup_menu` 在菜单关闭前不返回（模态阻塞）。
    pub(super) fn popup(
        app: &AppHandle,
        request: ClipboardItemMenuRequest,
        menu: &crate::settings::Menu,
    ) -> Result<()> {
        let state = app.try_state::<ClipboardItemMenuState>().ok_or_else(|| {
            AppError::Other(anyhow::anyhow!("ClipboardItemMenuState not managed"))
        })?;
        *state.target_item_id.lock().unwrap() = Some(request.item_id.clone());

        let window = app
            .get_webview_window(CLIPBOARD_WINDOW_LABEL)
            .ok_or_else(|| AppError::Other(anyhow::anyhow!("clipboard window missing")))?;

        let app_for_main = app.clone();
        let window_for_main = window.clone();
        let lang = crate::i18n::current_language(app);
        let menu_for_main = menu.clone();
        window
            .run_on_main_thread(move || {
                let menu = match build_menu(&app_for_main, &request, lang, &menu_for_main) {
                    Ok(m) => m,
                    Err(err) => {
                        log::warn!("build clipboard item menu failed: {err}");
                        return;
                    }
                };

                let state = app_for_main.state::<ClipboardItemMenuState>();
                {
                    let mut current = state.current.lock().unwrap();
                    *current = Some(menu);
                }

                let guard = state.current.lock().unwrap();
                let menu_ref = guard
                    .as_ref()
                    .expect("menu just stored above and lock is held");
                if let Err(err) = window_for_main.popup_menu(menu_ref) {
                    log::warn!("popup clipboard item menu failed: {err}");
                }
            })
            .map_err(|err| {
                AppError::Other(anyhow::anyhow!(
                    "schedule popup_clipboard_item_menu on main thread failed: {err}"
                ))
            })
    }

    fn action_group(action: ClipboardMenuAction) -> usize {
        for (i, group) in ACTION_GROUPS.iter().enumerate() {
            if group.contains(&action) {
                return i;
            }
        }

        ACTION_GROUPS.len()
    }

    fn build_menu(
        app: &AppHandle,
        request: &ClipboardItemMenuRequest,
        lang: Language,
        menu: &crate::settings::Menu,
    ) -> Result<Menu<Wry>> {
        let mut active: HashSet<ClipboardMenuAction> =
            request.available_actions.iter().copied().collect();
        if !request.groups.is_empty() {
            active.insert(ClipboardMenuAction::MoveToGroup);
        }

        enum Entry {
            Item(MenuItem<Wry>),
            Submenu(Submenu<Wry>),
            Sep(PredefinedMenuItem<Wry>),
        }
        let mut entries: Vec<Entry> = Vec::new();
        let mut prev_group: Option<usize> = None;

        for action in visible_ordered_actions(menu) {
            if !active.contains(&action) {
                continue;
            }

            let group = action_group(action);

            if prev_group.is_some() && prev_group != Some(group) {
                let sep = PredefinedMenuItem::separator(app).context("build separator")?;
                entries.push(Entry::Sep(sep));
            }
            prev_group = Some(group);

            if action == ClipboardMenuAction::MoveToGroup {
                let submenu = build_group_submenu(
                    app,
                    &request.groups,
                    request.current_group_id.as_deref(),
                    action.label(
                        lang,
                        request.is_favorite,
                        request.is_pinned,
                        request.has_note,
                    ),
                )?;
                entries.push(Entry::Submenu(submenu));
                continue;
            }

            let item = MenuItem::with_id(
                app,
                action.id(),
                action.label(
                    lang,
                    request.is_favorite,
                    request.is_pinned,
                    request.has_note,
                ),
                true,
                action.accelerator(),
            )
            .with_context(|| format!("build menu item {}", action.id()))?;
            entries.push(Entry::Item(item));
        }

        // AI 占位：本 commit 无 AI 命令，展示为不可点击的菜单项。
        if !menu.ai_visible.is_empty() {
            if !entries.is_empty() {
                let sep = PredefinedMenuItem::separator(app).context("build separator")?;
                entries.push(Entry::Sep(sep));
            }

            let ai_item = MenuItem::with_id(
                app,
                "cim::ai::placeholder",
                "AI（待开发）",
                false,
                None::<&str>,
            )
            .context("build ai placeholder menu item")?;
            entries.push(Entry::Item(ai_item));
        }

        let refs: Vec<&dyn IsMenuItem<Wry>> = entries
            .iter()
            .map(|e| match e {
                Entry::Item(i) => i as &dyn IsMenuItem<Wry>,
                Entry::Submenu(s) => s as &dyn IsMenuItem<Wry>,
                Entry::Sep(s) => s as &dyn IsMenuItem<Wry>,
            })
            .collect();

        let mut builder = MenuBuilder::new(app);
        for r in refs {
            builder = builder.item(r);
        }
        builder
            .build()
            .context("build clipboard item menu")
            .map_err(Into::into)
    }

    fn build_group_submenu(
        app: &AppHandle,
        groups: &[ClipboardMenuGroup],
        current_group_id: Option<&str>,
        label: &str,
    ) -> Result<Submenu<Wry>> {
        let items = groups
            .iter()
            .map(|group| {
                let checked = current_group_id == Some(group.id.as_str());

                CheckMenuItem::with_id(
                    app,
                    move_group_id(&group.id),
                    group.name.as_str(),
                    true,
                    checked,
                    None::<&str>,
                )
                .with_context(|| format!("build move group menu item {}", group.id))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let refs: Vec<&dyn IsMenuItem<Wry>> = items
            .iter()
            .map(|item| item as &dyn IsMenuItem<Wry>)
            .collect();

        Submenu::with_items(app, label, true, &refs)
            .context("build move group submenu")
            .map_err(Into::into)
    }

    pub(super) fn handle_event(app: &AppHandle, menu_id: &str) {
        if !menu_id.starts_with(MENU_PREFIX) {
            return;
        }
        let Some(action) = ClipboardMenuAction::from_id(menu_id) else {
            log::warn!("unknown clipboard menu id: {menu_id}");
            return;
        };

        let Some(state) = app.try_state::<ClipboardItemMenuState>() else {
            log::warn!("ClipboardItemMenuState missing on menu event");
            return;
        };

        let item_id = state.target_item_id.lock().unwrap().clone();
        let Some(item_id) = item_id else {
            log::warn!("no target item id recorded for menu action {menu_id}");
            return;
        };

        let payload = MenuActionPayload {
            action,
            item_id,
            group_id: group_id_from_menu_id(menu_id),
        };
        let Some(main) = app.get_webview_window(CLIPBOARD_WINDOW_LABEL) else {
            log::warn!("clipboard window missing on clipboard menu dispatch");
            return;
        };
        if let Err(err) = main.emit(CLIPBOARD_MENU_ACTION_EVENT, payload) {
            log::warn!("emit {CLIPBOARD_MENU_ACTION_EVENT} failed: {err}");
        }
    }
}

// ============================================================================
// 跨平台入口
// ============================================================================

/// setup 阶段调用：macOS 注册 muda 菜单状态；Windows 由 [`super::context_window::init`]
/// 单独建窗，这里 no-op。
#[allow(unused_variables)]
pub fn init(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    native::init(app);
}

/// 在当前光标处弹出列表项右键菜单。`available_actions` / `is_favorite` 由
/// 前端传入（来自 `ClipboardItem.availableActions` 与当前状态字段）。
#[tauri::command]
pub async fn popup_clipboard_item_menu(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    settings: State<'_, crate::settings::SettingsStore>,
    input: PopupClipboardItemMenuInput,
) -> Result<()> {
    let pool = db.pool().await;
    let groups = crate::db::groups::list_groups(&pool)
        .await?
        .into_iter()
        .filter(|group| !group.is_hidden)
        .map(|group| ClipboardMenuGroup {
            id: group.id,
            name: group.name,
        })
        .collect::<Vec<_>>();
    let request = ClipboardItemMenuRequest {
        item_id: input.item_id,
        available_actions: input.available_actions,
        groups,
        current_group_id: input.current_group_id,
        is_favorite: input.is_favorite,
        is_pinned: input.is_pinned,
        has_note: input.has_note,
    };
    let menu = settings.snapshot().menu;

    #[cfg(target_os = "macos")]
    {
        native::popup(&app, request, &menu)
    }

    #[cfg(target_os = "windows")]
    {
        super::context_window::show_for_clipboard_item(&app, &request, &menu)
    }
}

/// 全局菜单事件分发入口（注册在 `on_menu_event` 上）。Windows 不经此路径。
#[allow(unused_variables)]
pub fn handle_menu_event(app: &AppHandle, menu_id: &str) {
    #[cfg(target_os = "macos")]
    native::handle_event(app, menu_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_ordered_actions_default_returns_all_fourteen() {
        let menu = crate::settings::Menu::default();
        let actions = visible_ordered_actions(&menu);

        assert_eq!(actions.len(), 14);
        assert_eq!(
            actions,
            vec![
                ClipboardMenuAction::Paste,
                ClipboardMenuAction::PasteAsPlainText,
                ClipboardMenuAction::PasteAsPath,
                ClipboardMenuAction::Copy,
                ClipboardMenuAction::SaveImage,
                ClipboardMenuAction::OpenLink,
                ClipboardMenuAction::SendEmail,
                ClipboardMenuAction::RevealInFinder,
                ClipboardMenuAction::RevealInExplorer,
                ClipboardMenuAction::ToggleFavorite,
                ClipboardMenuAction::TogglePinned,
                ClipboardMenuAction::MoveToGroup,
                ClipboardMenuAction::EditNote,
                ClipboardMenuAction::Delete,
            ]
        );
    }

    #[test]
    fn visible_ordered_actions_filters_hidden() {
        let mut menu = crate::settings::Menu::default();
        menu.visible_actions = vec![
            ClipboardMenuAction::Paste,
            ClipboardMenuAction::Copy,
            ClipboardMenuAction::Delete,
        ];

        let actions = visible_ordered_actions(&menu);

        assert_eq!(
            actions,
            vec![
                ClipboardMenuAction::Paste,
                ClipboardMenuAction::Copy,
                ClipboardMenuAction::Delete
            ]
        );
    }

    #[test]
    fn visible_ordered_actions_respects_custom_order() {
        let mut menu = crate::settings::Menu::default();
        menu.order = vec![
            ClipboardMenuAction::Delete,
            ClipboardMenuAction::Paste,
            ClipboardMenuAction::Copy,
        ];

        let actions = visible_ordered_actions(&menu);

        assert_eq!(
            actions,
            vec![
                ClipboardMenuAction::Delete,
                ClipboardMenuAction::Paste,
                ClipboardMenuAction::Copy
            ]
        );
    }
}
