import { setAutostart, showTaskbarIcon } from "@/commands";
import { updateSettings } from "@/stores/settings";
import type { SettingsPatch } from "@/types/settings";
import type {
  PreferenceSetting,
  SettingValue,
  SortableCheckboxTreeSettingValue,
} from "../types/preferences";

/**
 * 提交单个设置变更，同时处理与该设置绑定的系统副作用。
 */
export async function commitSettingChange(
  setting: PreferenceSetting,
  value: SettingValue,
) {
  if (!setting.path) return;

  await applySideEffects(setting, value);
  await updateSettings(buildSettingPatch(setting, value));
}

/**
 * 比较设置值是否真正发生变化，避免无意义保存。
 */
export function settingValuesEqual(left: SettingValue, right: SettingValue) {
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right)) return false;
    if (left.length !== right.length) return false;

    return left.every((item, index) => {
      return item === right[index];
    });
  }

  if (typeof left === "object" || typeof right === "object") {
    if (typeof left !== "object" || typeof right !== "object") return false;
    if (left === null || right === null) return false;

    return JSON.stringify(left) === JSON.stringify(right);
  }

  return left === right;
}

/**
 * 把 schema path 和控件值转换成 Rust `update_settings` 可接收的深层补丁。
 */
function buildPatch(
  path: readonly string[],
  value: SettingValue,
): SettingsPatch {
  const [head, ...rest] = path;
  if (!head) return {};

  return {
    [head]: buildNestedPatch(rest, value),
  } as SettingsPatch;
}

/**
 * 按控件契约构造设置补丁；排序勾选树需要同时保存选择态与完整顺序，
 * 配置了 aiGroup 时一并写入 AI 动作分组的勾选与顺序。
 */
function buildSettingPatch(
  setting: PreferenceSetting,
  value: SettingValue,
): SettingsPatch {
  if (!setting.path) return {};

  if (
    setting.control.type === "sortableCheckboxTree" &&
    isSortableCheckboxTreeValue(value)
  ) {
    let patch = mergeSettingsPatch(
      buildPatch(setting.path, value.selected),
      buildPatch(setting.control.orderPath, value.order),
    );

    const { aiGroup } = setting.control;
    if (aiGroup && value.aiSelected) {
      const orderedAiSelected = value.aiOrder
        ? value.aiOrder.filter((id) => value.aiSelected?.includes(id))
        : value.aiSelected;

      patch = mergeSettingsPatch(
        patch,
        buildPatch(aiGroup.path, orderedAiSelected),
      );

      if (aiGroup.orderPath && value.aiOrder) {
        patch = mergeSettingsPatch(
          patch,
          buildPatch(aiGroup.orderPath, value.aiOrder),
        );
      }
    }

    return patch;
  }

  return buildPatch(setting.path, value);
}

/**
 * 判断控件提交值是否包含选择态和完整排序。
 */
function isSortableCheckboxTreeValue(
  value: SettingValue,
): value is SortableCheckboxTreeSettingValue {
  if (typeof value !== "object" || value === null) return false;
  if (!("selected" in value) || !("order" in value)) return false;

  return Array.isArray(value.selected) && Array.isArray(value.order);
}

/**
 * 深度合并两个设置补丁，避免同一父路径下的字段互相覆盖。
 */
function mergeSettingsPatch(
  left: SettingsPatch,
  right: SettingsPatch,
): SettingsPatch {
  return mergePatchValue(left, right) as SettingsPatch;
}

/**
 * 递归合并普通对象；数组按设置语义整体替换。
 */
function mergePatchValue(left: unknown, right: unknown): unknown {
  if (!isPlainObject(left) || !isPlainObject(right)) return right;

  const merged: Record<string, unknown> = { ...left };

  for (const [key, value] of Object.entries(right)) {
    merged[key] = mergePatchValue(merged[key], value);
  }

  return merged;
}

/**
 * 判断值是否为可递归合并的普通对象。
 */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * 递归构造嵌套补丁，数组字段保持整体替换语义。
 */
function buildNestedPatch(
  path: readonly string[],
  value: SettingValue,
): SettingValue | Record<string, unknown> {
  const [head, ...rest] = path;
  if (!head) return value;

  return {
    [head]: buildNestedPatch(rest, value),
  };
}

/**
 * 对需要立即触达系统层的设置执行配套副作用。
 */
async function applySideEffects(
  setting: PreferenceSetting,
  value: SettingValue,
) {
  if (setting.id === "control.autoStart") {
    await setAutostart(Boolean(value));
    return;
  }

  if (setting.id === "control.dockIcon") {
    await showTaskbarIcon(Boolean(value));
  }
}
