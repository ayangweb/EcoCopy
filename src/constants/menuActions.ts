import type { ClipboardKind } from "@/types/clipboard";
import type { AiInputKind, MenuAction } from "@/types/settings";

/**
 * AI 动作输入类型与条目类型的匹配规则：图片动作只用于图片条目，
 * 文本动作只用于文本条目（文件条目没有可加工的内容，无 AI 动作）。
 */
export function aiActionMatchesItemKind(
  inputKind: AiInputKind,
  itemKind: ClipboardKind,
) {
  return (
    (inputKind === "image" && itemKind === "image") ||
    (inputKind === "text" && itemKind === "text")
  );
}

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
