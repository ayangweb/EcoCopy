/* @unocss-include */
import type { PreferenceTabId } from "./types/preferences";

export const APP_NAME_PLACEHOLDER = "EcoPaste";

export const STORAGE_WARNING_BYTES = 1024 * 1024 * 1024;
export const STORAGE_ERROR_BYTES = 2 * 1024 * 1024 * 1024;

interface PreferenceTabMeta {
  activeClass: string;
  icon: string;
}

export const PREFERENCE_TAB_META: Record<PreferenceTabId, PreferenceTabMeta> = {
  about: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:info",
  },
  ai: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:sparkles",
  },
  data: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:database",
  },
  organize: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:history",
  },
  record: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:clipboard-plus",
  },
  reuse: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:mouse-pointer-click",
  },
  shortcuts: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:keyboard",
  },
  workflow: {
    activeClass: "bg-ant-fill-secondary text-ant-text",
    icon: "i-lucide:panel-top",
  },
};
