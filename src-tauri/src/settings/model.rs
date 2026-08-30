//! 设置数据模型。
//!
//! 每个字段都 `#[serde(default)]`，缺字段时回落到 `Default`，这样新增字段不破坏旧配置文件。

use serde::{Deserialize, Serialize};

use crate::db::models::ClipboardItemSort;
use crate::menu::clipboard_item::ClipboardMenuAction;

pub const WINDOW_OPEN_SELECTION_PRESERVE: &str = "preserve";
pub const WINDOW_OPEN_SELECTION_ALL: &str = "all";
pub const WINDOW_OPEN_GROUP_PREFIX: &str = "group:";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub general: General,
    pub appearance: Appearance,
    pub shortcuts: Shortcuts,
    pub clipboard: Clipboard,
    pub menu: Menu,
    pub onboarding: Onboarding,
    pub update: Update,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    pub auto_start: bool,
    /// Windows: persist the user's intent to run EcoPaste with administrator privileges.
    pub run_as_admin: bool,
    /// macOS 菜单栏 / Windows 系统托盘图标。
    pub tray_icon: bool,
    /// macOS Dock / Windows 任务栏图标。
    pub dock_icon: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            auto_start: false,
            run_as_admin: false,
            tray_icon: true,
            dock_icon: false,
        }
    }
}

/// 首次启动引导状态。业务数据仍由各自设置项持久化，本结构只记录引导进度。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Onboarding {
    pub completed: bool,
    pub last_step: u32,
    pub legacy_import: OnboardingLegacyImport,
}

/// 旧版数据导入的轻量状态记录；真实历史数据导入由 onboarding 导入流程负责。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct OnboardingLegacyImport {
    pub checked: bool,
    pub imported: bool,
    pub import_types: Vec<OnboardingLegacyImportType>,
    pub imported_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OnboardingLegacyImportType {
    Normal,
    Favorite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: Theme,
    pub language: Language,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::Auto,
            language: Language::ZhCN,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCN,
    #[serde(rename = "en-US")]
    EnUS,
}

impl Language {
    /// 把系统 locale（如 `zh_CN.UTF-8` / `en-US` / `ja-JP`）映射到支持的语言；
    /// 任何 zh-* 都归到 zh-CN，其余一律 en-US。
    pub fn from_system_locale(tag: &str) -> Self {
        let lower = tag.to_ascii_lowercase();
        if lower.starts_with("zh") {
            Self::ZhCN
        } else {
            Self::EnUS
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    /// 全局：唤起剪贴板窗口。
    pub open_clipboard: String,
    /// 全局：打开偏好设置窗口。
    pub open_preference: String,
    /// 仅 Windows：用 Win+V 唤起剪贴板窗口，替代系统剪贴板历史面板。默认关闭。
    pub win_v: bool,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            open_clipboard: "Alt+C".into(),
            open_preference: "Alt+X".into(),
            win_v: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Clipboard {
    pub capture: Capture,
    pub content: Content,
    pub display: Display,
    pub sensitive: Sensitive,
    pub history: History,
    pub search: Search,
    pub window: Window,
    pub preview: Preview,
    pub feedback: Feedback,
    pub filters: Filters,
}

/// 剪贴板内容类型采集开关。关闭后监听与手动读取都不入库对应类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Capture {
    pub text: bool,
    pub html: bool,
    pub rtf: bool,
    pub image: bool,
    pub files: bool,
    /// 文本最大收录大小，单位 MB。`0` = 不限制。
    pub max_text_mb: u32,
    /// 图片最大收录大小，单位 MB。`0` = 不限制。
    pub max_image_mb: u32,
    /// 剪贴板同时提供多种表示时的采集优先级。
    pub order: Vec<CaptureKind>,
}

impl Default for Capture {
    fn default() -> Self {
        Self {
            text: true,
            html: true,
            rtf: true,
            image: true,
            files: true,
            max_text_mb: 4,
            max_image_mb: 100,
            order: CaptureKind::default_order(),
        }
    }
}

impl Capture {
    /// 返回文本最大收录字节数；`None` 表示不限制。
    pub fn max_text_bytes(&self) -> Option<u64> {
        mb_to_bytes(self.max_text_mb)
    }

    /// 返回图片最大收录字节数；`None` 表示不限制。
    pub fn max_image_bytes(&self) -> Option<u64> {
        mb_to_bytes(self.max_image_mb)
    }

    /// 返回去重且补齐缺失项后的采集顺序，避免配置文件里手改出重复项后影响读取。
    pub fn ordered_kinds(&self) -> Vec<CaptureKind> {
        let mut order = Vec::new();
        for kind in self
            .order
            .iter()
            .copied()
            .chain(CaptureKind::default_order())
        {
            if !order.contains(&kind) {
                order.push(kind);
            }
        }

        order
    }

    /// 判断某个采集类型当前是否开启。
    pub fn is_enabled(&self, kind: CaptureKind) -> bool {
        match kind {
            CaptureKind::Text => self.text,
            CaptureKind::Html => self.html,
            CaptureKind::Rtf => self.rtf,
            CaptureKind::Image => self.image,
            CaptureKind::Files => self.files,
        }
    }
}

/// 把用户设置的 MB 值转换为字节阈值；`0` 表示不限。
fn mb_to_bytes(mb: u32) -> Option<u64> {
    if mb == 0 {
        return None;
    }

    Some(u64::from(mb) * 1024 * 1024)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureKind {
    Files,
    Image,
    Html,
    Rtf,
    Text,
}

impl CaptureKind {
    /// 默认顺序保持历史硬编码语义：文件 > 图片 > HTML > RTF > 纯文本。
    pub fn default_order() -> Vec<Self> {
        vec![Self::Files, Self::Image, Self::Html, Self::Rtf, Self::Text]
    }
}

/// 隐私保护设置。命中规则的内容可分别控制是否收录、是否脱敏展示。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Sensitive {
    /// 命中高置信密钥 / Token 时是否保存到历史记录。
    pub collect_secrets: bool,
    /// 已保存的敏感内容是否在列表与预览中脱敏展示。
    pub redact_secrets: bool,
}

impl Default for Sensitive {
    fn default() -> Self {
        Self {
            collect_secrets: true,
            redact_secrets: true,
        }
    }
}

/// 应用过滤规则。
/// `excluded_app_ids` 命中复制来源时，对应剪贴板内容不入库。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Filters {
    pub excluded_app_ids: Vec<String>,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            excluded_app_ids: default_excluded_app_ids(),
        }
    }
}

fn default_excluded_app_ids() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        // 系统级密码 / 密钥工具：用户从这里复制的几乎都是敏感凭据，默认不入库。
        // - com.apple.keychainaccess：钥匙串访问
        // - com.apple.Passwords：macOS 15 起的「密码」App
        vec![
            "com.apple.keychainaccess".to_owned(),
            "com.apple.Passwords".to_owned(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        // Windows 无系统内置的密码管理 App（凭据管理器是 Control Panel 子项，不会作为复制来源）。
        // 第三方密码管理器（1Password / Bitwarden / KeePass 等）因人而异，留给用户在 UI 勾选。
        Vec::new()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn macos_defaults_keep_sensitive_system_apps_excluded() {
        let ids = default_excluded_app_ids();

        assert!(ids.contains(&"com.apple.keychainaccess".to_owned()));
        assert!(ids.contains(&"com.apple.Passwords".to_owned()));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Content {
    /// 点击列表项时的自动粘贴行为。
    pub auto_paste: AutoPaste,
    /// 中键点击列表项时执行的动作。
    pub middle_click: MiddleClickAction,
    /// 复制（写回剪贴板）时去除格式。
    pub copy_plain: bool,
    /// 从历史复制后隐藏剪贴板窗口。
    pub copy_then_hide_window: bool,
    /// 粘贴时去除格式。
    pub paste_plain: bool,
    /// 粘贴文件记录时，默认写入路径文本而不是文件本身。
    pub paste_files_as_path: bool,
    /// 鼠标悬停时显示原始内容预览（HTML/RTF 渲染前的原文）。
    pub show_original_preview: bool,
    /// 删除普通条目前是否需要二次确认；收藏 / 置顶条目由各自确认开关单独控制。
    pub delete_confirm: bool,
    /// 是否允许删除收藏条目；关闭时收藏条目不显示删除入口。
    pub delete_favorite_items: bool,
    /// 删除收藏条目前是否需要二次确认。
    pub delete_favorite_confirm: bool,
    /// 是否允许删除置顶条目；关闭时置顶条目不显示删除入口。
    pub delete_pinned_items: bool,
    /// 删除置顶条目前是否需要二次确认。
    pub delete_pinned_confirm: bool,
    /// 开启后已收藏条目仅能在收藏分组删除，普通条目不受影响。
    pub delete_favorite_items_only_in_favorite_group: bool,
    pub auto_favorite: bool,
    /// 从历史中复制 / 粘贴时，是否刷新使用次数与 `updated_at`。
    pub update_on_reuse: bool,
    /// 历史列表默认排序，和 `ClipboardItemQuery.sort` 使用同一套契约字面量。
    pub sort: ClipboardItemSort,
    /// 列表项悬停操作按钮（仅保存已启用项，顺序按 `item_action_order` 过滤）。
    pub item_actions: Vec<ItemAction>,
    /// 列表项悬停操作按钮的完整排序，包含未启用项，供偏好弹框下次打开时恢复位置。
    pub item_action_order: Vec<ItemAction>,
}

impl Default for Content {
    fn default() -> Self {
        Self {
            auto_paste: AutoPaste::DoubleClickPaste,
            middle_click: MiddleClickAction::Disabled,
            copy_plain: false,
            copy_then_hide_window: false,
            paste_plain: false,
            paste_files_as_path: false,
            show_original_preview: true,
            delete_confirm: true,
            delete_favorite_items: false,
            delete_favorite_confirm: true,
            delete_pinned_items: false,
            delete_pinned_confirm: true,
            delete_favorite_items_only_in_favorite_group: true,
            auto_favorite: false,
            update_on_reuse: false,
            sort: ClipboardItemSort::UpdatedAt,
            item_actions: vec![
                ItemAction::Copy,
                ItemAction::Star,
                ItemAction::PinItem,
                ItemAction::Delete,
            ],
            item_action_order: vec![
                ItemAction::Paste,
                ItemAction::PastePlain,
                ItemAction::PastePath,
                ItemAction::Copy,
                ItemAction::CopyPlain,
                ItemAction::OpenLink,
                ItemAction::SendEmail,
                ItemAction::Reveal,
                ItemAction::Note,
                ItemAction::Star,
                ItemAction::PinItem,
                ItemAction::Delete,
            ],
        }
    }
}

/// 历史列表里不同内容类型的展示上限。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Display {
    /// 文本摘要最多显示行数。
    pub text_max_lines: u8,
    /// 图片缩略图显示高度，单位 px。
    pub image_max_height: u16,
    /// 文件列表最多返回并显示的条目数。
    pub file_max_count: u8,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            text_max_lines: 3,
            image_max_height: 64,
            file_max_count: 3,
        }
    }
}

impl Display {
    /// 返回主列表文件条目上限，并夹在 UI 支持的范围内控制 IPC 与 icon 抽取成本。
    pub fn file_entry_limit(self) -> usize {
        usize::from(self.file_max_count.clamp(1, 5))
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutoPaste {
    /// 点击只选中，不自动执行动作。
    Disabled,
    SingleClickPaste,
    #[default]
    DoubleClickPaste,
    SingleClickCopy,
    DoubleClickCopy,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MiddleClickAction {
    /// 中键点击仅选中，不自动执行动作。
    #[default]
    Disabled,
    SingleClickPaste,
    SingleClickPastePlain,
    SingleClickCopy,
    SingleClickCopyPlain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemAction {
    Paste,
    PastePlain,
    PastePath,
    Copy,
    CopyPlain,
    OpenLink,
    SendEmail,
    Reveal,
    Note,
    Star,
    PinItem,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Preview {
    pub hover_enabled: bool,
    pub hover_delay_ms: PreviewHoverDelayMs,
    pub space_enabled: bool,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            hover_enabled: false,
            hover_delay_ms: PreviewHoverDelayMs::Ms500,
            space_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewHoverDelayMs {
    Ms300,
    #[default]
    Ms500,
    Ms1000,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct History {
    pub retention: Retention,
    /// 最多保留条数。`0` = 不限。
    pub max_count: u32,
    /// 自动清理周期（小时）。`0` = 关闭周期清理，但启动时仍清理一次。
    pub cleanup_interval_hours: u32,
}

/// 历史保留时长。`unit = Forever` 时忽略 `value`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Retention {
    pub value: u32,
    pub unit: RetentionUnit,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            value: 0,
            unit: RetentionUnit::Forever,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RetentionUnit {
    Hours,
    Days,
    Weeks,
    Months,
    #[default]
    Forever,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Search {
    /// 剪贴板窗口每次显示时自动聚焦搜索框。
    pub default_focus: bool,
    /// 剪贴板窗口隐藏时清空搜索关键词。
    pub clear_on_hide: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            default_focus: false,
            clear_on_hide: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Window {
    pub position: WindowPosition,
    /// 打开剪贴板窗口时把历史列表回到顶部。
    pub scroll_to_top_on_open: bool,
    /// 打开剪贴板窗口时切换到指定范围；`Preserve` 表示保持上次状态。
    pub select_range_on_open: WindowOpenRangeSelection,
    /// 打开剪贴板窗口时切换到指定分类；`Preserve` 表示保持上次状态。
    pub select_category_on_open: WindowOpenCategorySelection,
    /// 打开剪贴板窗口时切换到指定自定义分组；可为 preserve / all / group:<id>。
    pub select_group_on_open: String,
    /// 隐藏窗口轻量化：剪贴板窗口隐藏后进入 dormant，非剪贴板窗口空闲后释放 WebView。
    pub lightweight_mode: bool,
    /// 非剪贴板窗口隐藏后释放 WebView 的空闲秒数。
    pub idle_destroy_seconds: u32,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            position: WindowPosition::FollowCursor,
            scroll_to_top_on_open: true,
            select_range_on_open: WindowOpenRangeSelection::Preserve,
            select_category_on_open: WindowOpenCategorySelection::Preserve,
            select_group_on_open: WINDOW_OPEN_SELECTION_PRESERVE.to_owned(),
            lightweight_mode: true,
            idle_destroy_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WindowOpenRangeSelection {
    #[default]
    Preserve,
    All,
    Favorite,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WindowOpenCategorySelection {
    #[default]
    Preserve,
    All,
    Text,
    Image,
    Files,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WindowPosition {
    Remember,
    #[default]
    FollowCursor,
    Center,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Feedback {
    pub copy_sound: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Update {
    pub auto_check: bool,
    pub frequency: UpdateFrequency,
    pub include_beta: bool,
    pub include_nightly: bool,
    pub last_checked_at: Option<String>,
    pub skipped_version: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateFrequency {
    #[default]
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Menu {
    pub visible_actions: Vec<ClipboardMenuAction>,
    pub order: Vec<ClipboardMenuAction>,
    pub ai_visible: Vec<String>,
    pub ai_order: Vec<String>,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            visible_actions: vec![
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
            ],
            order: vec![
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
            ],
            ai_visible: Vec::new(),
            ai_order: Vec::new(),
        }
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    #[test]
    fn menu_default_contains_all_fourteen_actions() {
        let menu = Menu::default();

        assert_eq!(menu.visible_actions.len(), 14);
        assert_eq!(menu.order.len(), 14);
    }

    #[test]
    fn menu_default_order_matches_action_groups() {
        let menu = Menu::default();
        let expected = vec![
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

        assert_eq!(menu.order, expected);
    }

    #[test]
    fn menu_deserialize_missing_field_uses_default() {
        let json = r#"{}"#;
        let menu: Menu = serde_json::from_str(json).unwrap();

        assert_eq!(menu, Menu::default());
    }

    #[test]
    fn settings_deserialize_missing_menu_uses_default() {
        let json = r#"{"general":{},"appearance":{},"shortcuts":{},"clipboard":{},"onboarding":{},"update":{}}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();

        assert_eq!(settings.menu, Menu::default());
    }
}
