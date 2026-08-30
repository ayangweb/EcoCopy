import type { MenuAction } from "@/types/settings";

/**
 * 右键菜单可选动作列表，供 `sortableCheckboxTree` 使用。
 * 顺序对齐 Rust `ClipboardMenuAction` 默认顺序。
 */
export const MENU_ACTION_OPTIONS: { value: MenuAction }[] = [
  { value: "paste" },
  { value: "pasteAsPlainText" },
  { value: "pasteAsPath" },
  { value: "copy" },
  { value: "saveImage" },
  { value: "openLink" },
  { value: "sendEmail" },
  { value: "revealInFinder" },
  { value: "revealInExplorer" },
  { value: "toggleFavorite" },
  { value: "togglePinned" },
  { value: "moveToGroup" },
  { value: "editNote" },
  { value: "delete" },
];
