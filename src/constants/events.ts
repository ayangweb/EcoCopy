/**
 * 与 Rust `app.emit` 事件名一一对应的常量表。
 * 命名采用 `domain://action` 形式（见 AGENTS.md）。
 */
export const TAURI_EVENT = {
  AI_CHUNK: "ai://chunk",
  AI_DONE: "ai://done",
  AI_ERROR: "ai://error",
  BACKUP_RECEIVED: "backup://received",
  CLIPBOARD_GROUPS_UPDATED: "clipboard-groups://updated",
  CLIPBOARD_MENU_ACTION: "clipboard://menu-action",
  CLIPBOARD_UPDATED: "clipboard://updated",
  CONTEXT_MENU_SHOW: "context-menu://show",
  CONTEXT_SUBMENU_SHOW: "context-submenu://show",
  KEYBOARD_NAV: "keyboard://nav",
  PREFERENCE_HIGHLIGHT_SETTING: "preference://highlight-setting",
  PREVIEW_UPDATED: "preview://updated",
  SETTINGS_UPDATED: "settings://updated",
  UPDATE_PROGRESS: "update://progress",
  WINDOW_BEFORE_DESTROY: "window://before-destroy",
  WINDOW_LIFECYCLE: "window://lifecycle",
  WINDOW_VISIBILITY: "window://visibility",
} as const;

export type TauriEvent = (typeof TAURI_EVENT)[keyof typeof TAURI_EVENT];
