import type { RetentionUnit, Settings } from "@/types/settings";

export type PreferenceTabId =
  | "record"
  | "organize"
  | "reuse"
  | "workflow"
  | "shortcuts"
  | "data"
  | "about"
  | "ai";

export interface RetentionSettingValue {
  unit: RetentionUnit;
  value: number;
}

export interface SortableCheckboxTreeSettingValue {
  order: string[];
  selected: string[];
  /** AI 动作分组（control.aiGroup 存在时）：完整顺序与勾选集。 */
  aiOrder?: string[];
  aiSelected?: string[];
}

export type SettingValue =
  | boolean
  | number
  | string
  | string[]
  | SortableCheckboxTreeSettingValue
  | RetentionSettingValue;

export type PreferenceStorageState = "loading" | "ready" | "error";

export interface PreferenceOption {
  value: string | number;
}

/** AI 动作分组的设置写入路径；orderPath 缺省 = 勾选顺序即保存顺序。 */
export interface PreferenceAiGroup {
  orderPath?: readonly string[];
  path: readonly string[];
}

export interface PreferenceShortcutTag {
  keys: string[];
}

export type PreferenceControl =
  | { type: "switch" }
  | {
      type: "permission";
      kind: "accessibility" | "fullDiskAccess" | "runAsAdministrator";
    }
  | { type: "segmented"; options: PreferenceOption[] }
  | { type: "select"; options: PreferenceOption[]; mode?: "multiple" }
  | { type: "clipboardGroupSelect" }
  | {
      type: "sortableTree";
      options: PreferenceOption[];
    }
  | {
      type: "sortableCheckboxTree";
      aiGroup?: PreferenceAiGroup;
      options: PreferenceOption[];
      orderPath: readonly string[];
    }
  | {
      type: "number";
      max?: number;
      min?: number;
      suffixKey?: string;
    }
  | { type: "retention" }
  | { type: "text" }
  | { type: "shortcutRecorder" }
  | { type: "textarea" }
  | { type: "appExclusion" }
  | { type: "action"; danger?: boolean }
  | { type: "sponsorQr" }
  | { type: "status" }
  | { type: "shortcutTags"; shortcuts: PreferenceShortcutTag[] };

export interface PreferenceSetting {
  control: PreferenceControl;
  disabled?: boolean;
  disabledWhen?: (settings: Settings) => boolean;
  id: string;
  keywords?: string[];
  parentId?: string;
  path?: readonly string[];
  status?: "comingSoon" | "alwaysOn" | "requiresBackend" | "experimental";
  value?: (settings: Settings) => SettingValue;
}

export type PreferenceSettingChangeHandler = (
  setting: PreferenceSetting,
  value: SettingValue,
) => Promise<void>;

export interface PreferenceSection {
  id: string;
  settings: PreferenceSetting[];
}

export interface PreferenceTab {
  icon: string;
  id: PreferenceTabId;
  sections: PreferenceSection[];
}
