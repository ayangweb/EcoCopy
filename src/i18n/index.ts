import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import aiEnUS from "@/locales/en-US/ai.json";
import clipboardEnUS from "@/locales/en-US/clipboard.json";
import commandsEnUS from "@/locales/en-US/commands.json";
import commonEnUS from "@/locales/en-US/common.json";
import onboardingEnUS from "@/locales/en-US/onboarding.json";
import preferencesEnUS from "@/locales/en-US/preferences.json";
import previewEnUS from "@/locales/en-US/preview.json";
import updateEnUS from "@/locales/en-US/update.json";
import aiZhCN from "@/locales/zh-CN/ai.json";
import clipboardZhCN from "@/locales/zh-CN/clipboard.json";
import commandsZhCN from "@/locales/zh-CN/commands.json";
import commonZhCN from "@/locales/zh-CN/common.json";
import onboardingZhCN from "@/locales/zh-CN/onboarding.json";
import preferencesZhCN from "@/locales/zh-CN/preferences.json";
import previewZhCN from "@/locales/zh-CN/preview.json";
import updateZhCN from "@/locales/zh-CN/update.json";
import type { Language } from "@/types/settings";

export const DEFAULT_LANGUAGE: Language = "zh-CN";
export const I18N_NAMESPACES = [
  "common",
  "commands",
  "clipboard",
  "ai",
  "onboarding",
  "preferences",
  "preview",
  "update",
] as const;

/**
 * react-i18next 初始化入口；资源来自 JSON，key 使用稳定语义路径。
 */
void i18n.use(initReactI18next).init({
  defaultNS: "common",
  fallbackLng: DEFAULT_LANGUAGE,
  interpolation: {
    escapeValue: false,
  },
  lng: DEFAULT_LANGUAGE,
  ns: I18N_NAMESPACES,
  resources: {
    "en-US": {
      ai: aiEnUS,
      clipboard: clipboardEnUS,
      commands: commandsEnUS,
      common: commonEnUS,
      onboarding: onboardingEnUS,
      preferences: preferencesEnUS,
      preview: previewEnUS,
      update: updateEnUS,
    },
    "zh-CN": {
      ai: aiZhCN,
      clipboard: clipboardZhCN,
      commands: commandsZhCN,
      common: commonZhCN,
      onboarding: onboardingZhCN,
      preferences: preferencesZhCN,
      preview: previewZhCN,
      update: updateZhCN,
    },
  },
});

export default i18n;
