//! 窗口声明式描述（descriptor）与静态 registry。
//!
//! 每个窗口只在这里声明一次；show / hide / close / 销毁重建等主流程统一查 registry，
//! 不再在各处散写 `if label == ...` 分支。新增窗口时在 [`DESCRIPTORS`] 加一条即可获得
//! 生命周期能力。

use tauri::AppHandle;

use super::super::{
    build_onboarding_window, build_preference_window, build_update_window, preview,
    CLIPBOARD_PREVIEW_WINDOW_LABEL, CLIPBOARD_WINDOW_LABEL, ONBOARDING_WINDOW_LABEL,
    PREFERENCE_WINDOW_LABEL, UPDATE_WINDOW_LABEL,
};
use crate::core::Result;

#[cfg(target_os = "windows")]
use crate::menu::context_window::{
    build_context_menu_window, build_context_submenu_window, CONTEXT_MENU_WINDOW_LABEL,
    CONTEXT_SUBMENU_WINDOW_LABEL,
};

/// 窗口保留 / 销毁策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainPolicy {
    /// 永久保留实例，隐藏后不销毁（main）。
    Permanent,
    /// 隐藏空闲超过用户设置秒数后销毁 WebView，打开时重建。
    DestroyWhenIdle,
}

impl RetainPolicy {
    /// 返回用于调试快照的稳定字面量。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::DestroyWhenIdle => "destroyWhenIdle",
        }
    }
}

/// 单个窗口的声明式描述。
#[derive(Clone, Copy)]
pub struct WindowDescriptor {
    /// Tauri window label。
    pub label: &'static str,
    /// 是否向前端广播生命周期事件（`window://lifecycle`）。
    pub emits_lifecycle: bool,
    /// 保留 / 销毁策略。
    pub retain_policy: RetainPolicy,
    /// 按需重建函数。`DestroyWhenIdle` 窗口被销毁后由各自打开入口用它重新建窗；
    /// `Permanent` 窗口无需重建，为 `None`。
    pub build: Option<fn(&AppHandle) -> Result<()>>,
}

/// 全部窗口的静态声明表。新增窗口在此追加。
static DESCRIPTORS: &[WindowDescriptor] = &[
    WindowDescriptor {
        label: CLIPBOARD_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::Permanent,
        build: None,
    },
    WindowDescriptor {
        label: PREFERENCE_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::DestroyWhenIdle,
        build: Some(build_preference_window),
    },
    WindowDescriptor {
        label: ONBOARDING_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::DestroyWhenIdle,
        build: Some(build_onboarding_window),
    },
    WindowDescriptor {
        label: UPDATE_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::DestroyWhenIdle,
        build: Some(build_update_window),
    },
    WindowDescriptor {
        label: CLIPBOARD_PREVIEW_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::DestroyWhenIdle,
        build: Some(preview::build_clipboard_preview_window),
    },
    #[cfg(target_os = "windows")]
    WindowDescriptor {
        label: CONTEXT_MENU_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::Permanent,
        build: Some(build_context_menu_window),
    },
    #[cfg(target_os = "windows")]
    WindowDescriptor {
        label: CONTEXT_SUBMENU_WINDOW_LABEL,
        emits_lifecycle: true,
        retain_policy: RetainPolicy::Permanent,
        build: Some(build_context_submenu_window),
    },
];

/// 按 label 查 descriptor；未登记的 label 返回 `None`。
pub fn descriptor_for(label: &str) -> Option<&'static WindowDescriptor> {
    DESCRIPTORS.iter().find(|d| d.label == label)
}

/// 返回所有窗口 descriptor，供生命周期调试快照遍历。
pub fn descriptors() -> &'static [WindowDescriptor] {
    DESCRIPTORS
}
