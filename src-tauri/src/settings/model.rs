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
    pub ai: Ai,
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
#[serde(rename_all = "camelCase")]
pub struct AiModelProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_true")]
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActionTemplate {
    pub id: String,
    pub name: String,
    pub input_kind: crate::ai::AiInputKind,
    pub prompt: String,
    pub model_profile_id: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Ai {
    pub enabled: bool,
    pub auto_writeback: bool,
    pub models: Vec<AiModelProfile>,
    pub default_model_id: Option<String>,
    pub quick_actions: Vec<String>,
    pub custom_templates: Vec<AiActionTemplate>,
    pub disabled_actions: Vec<String>,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_writeback: true,
            models: Vec::new(),
            default_model_id: None,
            quick_actions: Vec::new(),
            custom_templates: default_ai_templates(),
            disabled_actions: Vec::new(),
        }
    }
}

/// 4 个默认 AI 动作模板（中文名称，用户可在管理弹窗中编辑、删除或新增）：
/// 文本「翻译 / 总结」+ 图片「图片 OCR / 描述图片」。提示词只描述动作，
/// 文本或图片输入由 `ai::prompt` 按固定边界自动附加。
fn default_ai_templates() -> Vec<AiActionTemplate> {
    use crate::ai::AiInputKind;

    vec![
        AiActionTemplate {
            id: "translate".into(),
            name: "翻译".into(),
            input_kind: AiInputKind::Text,
            prompt: "你是一位专业的翻译引擎。将输入内容翻译为中文。只输出译文本身，不要任何解释、引号或额外内容。若内容已经是中文，原样输出。保留原文的段落与换行结构；代码、命令、URL、邮箱和专有名词保持原样。若输入中残留 HTML 等格式标记，忽略标记只翻译正文文字，输出干净的纯文本。".into(),
            model_profile_id: None,
        },
        AiActionTemplate {
            id: "summarize".into(),
            name: "总结".into(),
            input_kind: AiInputKind::Text,
            prompt: "你把以下文本提炼成结构化要点总结：先用一句话概括主旨，再分条列出关键要点，语言与原文一致。只输出总结本身，不要任何解释、前言或收尾客套。保留原文中的关键数字、专有名词与结论；若原文为分点内容，合并同类项后仍以条目呈现。".into(),
            model_profile_id: None,
        },
        AiActionTemplate {
            id: "ocr".into(),
            name: "图片 OCR".into(),
            input_kind: AiInputKind::Image,
            prompt: "你是 OCR 引擎。识别图片中的所有文字，按原始版面顺序输出，保留原始换行与段落结构。只输出识别到的文字本身，禁止臆造、翻译、解释或添加注释；模糊或无法确认的字以［？］占位。".into(),
            model_profile_id: None,
        },
        AiActionTemplate {
            id: "describeImage".into(),
            name: "描述图片".into(),
            input_kind: AiInputKind::Image,
            prompt: "你负责看图并描述：先一句话概括图片主体与场景，再按需补充关键细节（文字内容、数据、界面元素等）。若图中有文字请一并完整转写。语言跟随图片内容的主要语言，只输出描述本身，不要任何解释或开场白。".into(),
            model_profile_id: None,
        },
    ]
}

/// 历史出厂模板指纹（id, 名称, 输入类型, 提示词；绑定档案出厂恒为 None）。
/// 模板是普通用户数据：只有集合与内容**全字段**等于某代出厂预置时才视为
/// 「用户未修改」，避免迁移静默重置只改过提示词/名称的用户编辑。
type PresetFingerprint = [(
    &'static str,
    &'static str,
    crate::ai::AiInputKind,
    &'static str,
)];

/// 旧版出厂默认（4 个纯文本、短提示词）。
const LEGACY_PRESET_TEMPLATES: &PresetFingerprint = &[
    (
        "translate",
        "翻译",
        crate::ai::AiInputKind::Text,
        "请将以下内容翻译为中文",
    ),
    (
        "polish",
        "润色",
        crate::ai::AiInputKind::Text,
        "请润色以下文本，使其更流畅、专业",
    ),
    (
        "summarize",
        "总结",
        crate::ai::AiInputKind::Text,
        "请总结以下内容的核心要点",
    ),
    (
        "explain",
        "解释",
        crate::ai::AiInputKind::Text,
        "请用通俗易懂的语言解释以下内容",
    ),
];

/// 中间开发版本曾把默认集临时扩到 8 个；同样按指纹识别未修改的集合后收敛。
const EIGHT_PRESET_TEMPLATES: &PresetFingerprint = &[
    (
        "translate",
        "翻译",
        crate::ai::AiInputKind::Text,
        "你是一位专业的翻译引擎。将输入内容翻译为中文。只输出译文本身，不要任何解释、引号或额外内容。若内容已经是中文，原样输出。保留原文的段落与换行结构；代码、命令、URL、邮箱和专有名词保持原样。若输入中残留 HTML 等格式标记，忽略标记只翻译正文文字，输出干净的纯文本。",
    ),
    (
        "polish",
        "润色",
        crate::ai::AiInputKind::Text,
        "你负责润色/改写以下文本：保留原意与原语言，去除 AI 味与口水，使表达更自然精练。保留原文的段落结构，只输出结果本身，不要任何解释。",
    ),
    (
        "summarize",
        "总结",
        crate::ai::AiInputKind::Text,
        "你把以下文本提炼成结构化要点总结，语言与原文一致，分条列出。只输出总结本身，不要任何解释或前言。",
    ),
    (
        "explain",
        "解释",
        crate::ai::AiInputKind::Text,
        "你作为通用答疑助手，用通俗语言解释用户提出的内容或问题。只输出解释本身，不要开场白和收尾客套。",
    ),
    (
        "mouthpiece",
        "代回消息",
        crate::ai::AiInputKind::Text,
        "你帮用户回消息：把下面的语境与内容变成得体的私发回复，语气自然，语言与消息一致，不要引号，只输出回复内容。",
    ),
    (
        "ocr",
        "图片 OCR",
        crate::ai::AiInputKind::Image,
        "你是 OCR 引擎。识别图片中的所有文字，保留原始换行与段落。只输出识别到的文字本身，禁止臆造、解释或注释。",
    ),
    (
        "describeImage",
        "描述图片",
        crate::ai::AiInputKind::Image,
        "你看图并描述内容；若图中有文字请一并转写。只输出描述本身，不要任何解释。",
    ),
    (
        "translateImage",
        "翻译图片",
        crate::ai::AiInputKind::Image,
        "你先识别图片中的文字，再翻译为中文。输出原文对照与译文两部分，不要额外解释。",
    ),
];

/// 判断模板集合是否恰好等于某代出厂预置（数量与全字段一致，顺序无关）。
fn matches_preset(custom: &[AiActionTemplate], preset: &PresetFingerprint) -> bool {
    custom.len() == preset.len()
        && preset.iter().all(|(id, name, kind, prompt)| {
            custom.iter().any(|tpl| {
                tpl.id == *id
                    && tpl.name == *name
                    && tpl.input_kind == *kind
                    && tpl.prompt == *prompt
                    && tpl.model_profile_id.is_none()
            })
        })
}

/// 迁移：出厂预置集合变化时原地收敛到当前默认（老 4 文本集 / 中间 8 集合 → 新 4 集）。
/// 用户增删或改过任何模板内容（指纹不等于任何预置）时不动，避免覆盖用户配置。
/// 同步清理派生集合（菜单勾选/排序、hover 快捷、禁用列表）里滞留的已移除预置 id，
/// 避免「模板删了、联动配置还在」的脏数据写进 settings.json。
pub fn migrate_legacy_default_templates(settings: &mut Settings) {
    let is_preset_set = matches_preset(&settings.ai.custom_templates, LEGACY_PRESET_TEMPLATES)
        || matches_preset(&settings.ai.custom_templates, EIGHT_PRESET_TEMPLATES);

    if !is_preset_set {
        return;
    }

    log::info!("migrating preset AI templates to current default set (4)");
    settings.ai.custom_templates = default_ai_templates();

    let valid_ids: std::collections::HashSet<String> = settings
        .ai
        .custom_templates
        .iter()
        .map(|tpl| tpl.id.clone())
        .collect();

    settings.menu.ai_visible.retain(|id| valid_ids.contains(id));
    settings.menu.ai_order.retain(|id| valid_ids.contains(id));
    settings
        .ai
        .quick_actions
        .retain(|id| valid_ids.contains(id));
    settings
        .ai
        .disabled_actions
        .retain(|id| valid_ids.contains(id));
}

impl Ai {
    pub fn default_model(&self) -> Option<&AiModelProfile> {
        self.default_model_id
            .as_deref()
            .and_then(|id| self.models.iter().find(|p| p.id == id))
            .or_else(|| self.models.first())
    }
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

    /// 按指纹表构造历史出厂模板（含真实历史提示词）。
    fn preset_template(
        (id, name, kind, prompt): &(&str, &str, crate::ai::AiInputKind, &str),
    ) -> AiActionTemplate {
        AiActionTemplate {
            id: (*id).into(),
            name: (*name).into(),
            input_kind: *kind,
            prompt: (*prompt).into(),
            model_profile_id: None,
        }
    }

    #[test]
    fn migration_upgrades_untouched_legacy_template_set() {
        let mut settings = Settings::default();
        // 模拟老版本落盘的默认 4 模板，并给派生集合塞入将被移除的预置 id。
        settings.ai.custom_templates = LEGACY_PRESET_TEMPLATES
            .iter()
            .map(preset_template)
            .collect();
        settings.menu.ai_visible = vec!["polish".into(), "ocr".into()];
        settings.menu.ai_order = vec!["translateImage".into()];
        settings.ai.quick_actions = vec!["mouthpiece".into()];
        settings.ai.disabled_actions = vec!["explain".into()];

        migrate_legacy_default_templates(&mut settings);

        // 模板收敛到新默认 4 集，派生集合里的陈旧预置 id 一并清理。
        assert_eq!(settings.ai.custom_templates.len(), 4);
        let ids: Vec<&str> = settings
            .ai
            .custom_templates
            .iter()
            .map(|tpl| tpl.id.as_str())
            .collect();
        assert_eq!(ids, ["translate", "summarize", "ocr", "describeImage"]);
        // polish 已移除被清掉；ocr 仍在新默认集中，正确保留。
        assert_eq!(settings.menu.ai_visible, ["ocr"]);
        assert!(settings.menu.ai_order.is_empty());
        assert!(settings.ai.quick_actions.is_empty());
        assert!(settings.ai.disabled_actions.is_empty());
    }

    #[test]
    fn migration_converges_untouched_eight_template_set() {
        let mut settings = Settings::default();
        // 中间版本曾把默认集扩到 8 个；用户未改过时收敛回当前 4 默认。
        settings.ai.custom_templates = EIGHT_PRESET_TEMPLATES.iter().map(preset_template).collect();
        assert_eq!(settings.ai.custom_templates.len(), 8);

        migrate_legacy_default_templates(&mut settings);

        assert_eq!(settings.ai.custom_templates.len(), 4);
    }

    #[test]
    fn migration_skips_user_edited_legacy_template() {
        let mut settings = Settings::default();
        // id 集合仍是老出厂 4 个，但用户改过其中一条提示词 → 视为已编辑，不迁移。
        let mut templates: Vec<AiActionTemplate> = LEGACY_PRESET_TEMPLATES
            .iter()
            .map(preset_template)
            .collect();
        templates[0].prompt = "我自己调过的翻译提示词".into();
        settings.ai.custom_templates = templates;
        let before = settings.ai.custom_templates.clone();

        migrate_legacy_default_templates(&mut settings);

        assert_eq!(settings.ai.custom_templates, before);
    }

    #[test]
    fn migration_skips_user_modified_template_set() {
        let mut settings = Settings::default();
        settings.ai.custom_templates.push(AiActionTemplate {
            id: "custom:1".into(),
            name: "我的模板".into(),
            input_kind: crate::ai::AiInputKind::Text,
            prompt: "自定义".into(),
            model_profile_id: None,
        });
        let before = settings.ai.custom_templates.clone();

        migrate_legacy_default_templates(&mut settings);

        // 用户改过集合（4 默认 + 1 自定义）→ 模板与派生集合均不动。
        assert_eq!(settings.ai.custom_templates, before);
    }

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
