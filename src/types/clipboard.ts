/**
 * 与 Rust `db::models::ClipboardItem` 对应（`serde(rename_all = "camelCase")`）。
 * 列表视图下 text 类型条目的 `content` / `searchText` 由 Rust 端置空，渲染走 `summary`。
 */

export type ClipboardKind = "text" | "image" | "files";

export type ClipboardSubKind =
  | "rtf"
  | "html"
  | "url"
  | "email"
  | "color"
  | "path";

export type ClipboardPlatform = "macos" | "windows";

/**
 * 右键菜单可用动作。与 Rust `ClipboardAction` 对应（`serde(rename_all = "camelCase")`）。
 * 顺序由后端给出，前端按此顺序渲染。
 */
export type ClipboardAction =
  | "paste"
  | "pasteAsPlainText"
  | "pasteAsPath"
  | "copy"
  | "saveImage"
  | "openLink"
  | "sendEmail"
  | "revealInFinder"
  | "revealInExplorer"
  | "toggleFavorite"
  | "togglePinned"
  | "moveToGroup"
  | "editNote"
  | "delete"
  | "aiProcess";

export interface ClipboardItem {
  id: string;
  kind: ClipboardKind;
  subKind: ClipboardSubKind | null;
  groupId: string | null;
  sourceAppId: string | null;
  content: string;
  contentHash: string;
  searchText: string | null;
  summary: string | null;
  fileTypes: string | null;
  size: number | null;
  width: number | null;
  height: number | null;
  useCount: number;
  isFavorite: boolean;
  isPinned: boolean;
  isSensitive: boolean;
  platform: ClipboardPlatform;
  note: string | null;
  createdAt: string;
  updatedAt: string;
  /**
   * 后端 LEFT JOIN `clipboard_apps` 顺带返回的来源应用元信息。
   * `sourceAppIconPath` 已由 Rust 命令层用 `AppIconStore` 解析为绝对路径，
   * 前端直接 `convertFileSrc(sourceAppIconPath)` 渲染。
   * `sourceAppIconFile` 是 Rust 内部解析用的原始文件名，前端通常不需要。
   */
  sourceAppName?: string;
  sourceAppIconFile?: string;
  sourceAppIconPath?: string;
  /** image 条目的缩略图绝对路径（后端预处理返回）。 */
  imageThumbnailPath?: string;
  /** files 条目的预处理条目（与 content 中路径顺序一致，数量由 Rust 按设置截断）。 */
  fileEntries?: FileEntry[];
  /** 右键菜单可用动作；顺序由 Rust 给出，前端按序渲染并按 action 查文案/快捷键。 */
  availableActions?: ClipboardAction[];
  /** sub_kind = color 时由 Rust 校验过的 CSS 颜色串（合法才存在）。 */
  colorPreview?: string;
  /** Files 卡片渲染模式，由 Rust 命令层根据 `fileEntries` 计算。 */
  filesPreviewKind?: "imagePreview" | "list";
  /** Rust 按本地时区做三档格式化的 createdAt：HH:mm / MM-DD HH:mm / YYYY-MM-DD HH:mm。 */
  displayCreatedAt?: string;
}

export interface ClipboardApp {
  id: string;
  name: string;
  iconFile: string | null;
  iconPath: string | null;
  platform: ClipboardPlatform;
  createdAt: string;
  updatedAt: string;
}

export interface FileEntry {
  path: string;
  name: string;
  isDir: boolean;
  isImage: boolean;
  exists: boolean;
  iconPath?: string;
}

export type ClipboardItemSort =
  | "createdAtDesc"
  | "updatedAtDesc"
  | "useCountDesc";

export type ClipboardGroup = "all" | "text" | "image" | "files" | "favorite";

export type ClipboardRange = "all" | "favorite";

export type ClipboardCategory = ClipboardKind;

export type ClipboardGroupIcon = string;

export interface ClipboardGroupRecord {
  id: string;
  name: string;
  icon: ClipboardGroupIcon;
  isHidden: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface ClipboardGroupInput {
  name: string;
  icon: ClipboardGroupIcon;
  isHidden: boolean;
}

export interface ClipboardItemQuery {
  kind?: ClipboardKind;
  groupId?: string;
  favorite?: boolean;
  pinned?: boolean;
  /** 列表顶部 Tab；Rust 侧翻译成 kind / favorite，前端不再手动映射。 */
  group?: ClipboardGroup;
  keyword?: string;
  sort?: ClipboardItemSort;
  limit?: number;
  offset?: number;
}

/**
 * 列表查询的一页结果，Rust 直接给出 `total` 与 `hasMore`，
 * 前端无需再用 `length === pageSize` 近似。
 */
export interface ClipboardItemPage {
  list: ClipboardItem[];
  total: number;
  hasMore: boolean;
}

/**
 * 备注更新结果：Rust 归一化后的 note + 是否触发 auto-favorite。
 */
export interface UpdateNoteResult {
  note: string | null;
  autoFavorited: boolean;
}
