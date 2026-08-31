import type { TreeDataNode } from "antd";
import { Button } from "antd";
import type { TFunction } from "i18next";
import type { FC, Key } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSnapshot } from "valtio";
import Tooltip from "@/components/Tooltip";
import {
  isItemAction,
  resolveItemActionIcon,
  translateItemActionLabel,
} from "@/constants/itemActions";
import { settingsState } from "@/stores/settings";
import { cn } from "@/utils/cn";
import type {
  PreferenceOption,
  PreferenceSetting,
  SettingValue,
} from "../../types/preferences";
import {
  translatePreferenceControlLabel,
  translatePreferenceOption,
  translatePreferenceSetting,
} from "../../utils/preferenceI18n";
import SortableTreeModal from "./SortableTreeModal";
import type { ControlProps } from "./types";

interface SortableCheckboxTreeControlProps extends ControlProps {
  setting: PreferenceSetting;
  value?: SettingValue;
}

/** AI 分组头节点 key；不参与勾选与排序，仅作视觉分隔。 */
const AI_GROUP_HEADER_KEY = "__ai_group__";

/**
 * 用一个按钮打开可拖拽 Tree 弹框，勾选项按当前树顺序保存。
 * 配置了 aiGroup 的设置项会在静态动作之后追加"AI 动作"分组（受启用总闸控制）。
 */
const SortableCheckboxTreeControl: FC<SortableCheckboxTreeControlProps> = (
  props,
) => {
  const { t } = useTranslation("preferences");
  const { t: clipboardT } = useTranslation("clipboard");
  const { t: commonT } = useTranslation("common");
  const { disabled, onChange, setting, value } = props;
  const { ai } = useSnapshot(settingsState);
  const aiGroup =
    setting.control.type === "sortableCheckboxTree"
      ? setting.control.aiGroup
      : undefined;
  const aiEnabled = Boolean(ai?.enabled);
  const aiTemplates = ai?.customTemplates ?? [];
  const treeValue = resolveTreeValue(value);
  const aiValue = aiGroup
    ? resolveAiGroupValue(
        settingsState as unknown as Record<string, unknown>,
        aiGroup,
      )
    : { order: [], selected: [] };
  const selectedLabels = resolveSelectedLabels(
    t,
    clipboardT,
    setting,
    treeValue.selected,
    aiGroup ? aiValue.selected : [],
  );
  const controlLabel = translatePreferenceControlLabel(t, setting);
  // 纯 AI 动作树（静态选项为空）没有可摘要的静态文案，tooltip 只会重复
  // 列出勾选的模板，直接不显示；混合树（静态动作 + AI 组）保留摘要。
  const showSummaryTooltip =
    setting.control.type === "sortableCheckboxTree" &&
    setting.control.options.length > 0;
  const [open, setOpen] = useState(false);
  const [treeData, setTreeData] = useState<TreeDataNode[]>([]);
  const [checkedKeys, setCheckedKeys] = useState<Key[]>([]);

  if (setting.control.type !== "sortableCheckboxTree") return null;

  const openModal = () => {
    setTreeData(
      buildTreeData(t, clipboardT, setting, treeValue, {
        aiEnabled,
        aiGroup,
        aiOrder: aiValue.order,
        aiTemplates,
      }),
    );
    setCheckedKeys([
      ...treeValue.selected,
      ...(aiEnabled ? aiValue.selected : []),
    ]);
    setOpen(true);
  };

  const closeModal = () => {
    setOpen(false);
  };

  const handleSave = async (nextOrder: string[], nextCheckedKeys: string[]) => {
    const aiIds = new Set(aiTemplates.map((tpl) => tpl.id));
    const order = nextOrder.filter((key) => {
      return key !== AI_GROUP_HEADER_KEY && !aiIds.has(key);
    });
    const selected = nextCheckedKeys.filter((key) => !aiIds.has(key));

    if (!aiGroup) {
      await onChange(setting, { order, selected });
      setOpen(false);

      return;
    }

    // AI 关闭时分组禁用，保持已有配置不被覆盖。
    if (!aiEnabled) {
      await onChange(setting, {
        aiOrder: aiValue.order,
        aiSelected: aiValue.selected,
        order,
        selected,
      });
      setOpen(false);

      return;
    }

    await onChange(setting, {
      aiOrder: nextOrder.filter((key) => aiIds.has(key)),
      aiSelected: nextCheckedKeys.filter((key) => aiIds.has(key)),
      order,
      selected,
    });
    setOpen(false);
  };

  return (
    <>
      {showSummaryTooltip ? (
        <Tooltip title={selectedLabels}>
          <Button disabled={disabled} onClick={openModal}>
            {controlLabel}
          </Button>
        </Tooltip>
      ) : (
        <Button disabled={disabled} onClick={openModal}>
          {controlLabel}
        </Button>
      )}

      <SortableTreeModal
        cancelText={commonT("actions.cancel")}
        checkable
        checkedKeys={checkedKeys}
        okText={commonT("actions.save")}
        onCancel={closeModal}
        onSave={handleSave}
        open={open}
        title={translatePreferenceSetting(t, setting, "title")}
        treeData={treeData}
      />
    </>
  );
};

export default SortableCheckboxTreeControl;

/**
 * 解析排序勾选树控件值；数组输入视为同时包含选择态和排序。
 */
function resolveTreeValue(value?: SettingValue) {
  if (Array.isArray(value)) {
    return { order: value, selected: value };
  }

  if (typeof value !== "object" || value === null) {
    return { order: [], selected: [] };
  }

  if (!("selected" in value) || !("order" in value)) {
    return { order: [], selected: [] };
  }

  const order = resolveStringArray(value.order);
  const selected = resolveStringArray(value.selected);

  return {
    order: order.length > 0 ? order : selected,
    selected,
  };
}

/**
 * 从未知值中提取字符串数组。
 */
function resolveStringArray(value: unknown) {
  if (!Array.isArray(value)) return [];

  return value.filter((item) => {
    return typeof item === "string";
  });
}

/**
 * 从设置快照按路径读取字符串数组（AI 分组的勾选与排序值）。
 */
function resolveAiGroupValue(
  source: Record<string, unknown>,
  aiGroup: { orderPath?: readonly string[]; path: readonly string[] },
) {
  const selected = readStringPath(source, aiGroup.path);

  return {
    order: aiGroup.orderPath
      ? readStringPath(source, aiGroup.orderPath)
      : selected,
    selected,
  };
}

function readStringPath(
  source: unknown,
  path: readonly string[],
): Array<string> {
  let current: unknown = source;

  for (const key of path) {
    if (typeof current !== "object" || current === null) return [];

    current = (current as Record<string, unknown>)[key];
  }

  return resolveStringArray(current);
}

/**
 * 生成 Tooltip 里展示的已选动作摘要。
 */
function resolveSelectedLabels(
  t: TFunction<"preferences">,
  clipboardT: TFunction<"clipboard">,
  setting: PreferenceSetting,
  selectedValues: string[],
  aiSelectedValues: string[],
) {
  if (setting.control.type !== "sortableCheckboxTree") return "";

  const options = setting.control.options;
  const labels = selectedValues.map((value) => {
    const option = options.find((item) => {
      return String(item.value) === value;
    });
    if (!option) return value;

    return resolveOptionLabel(t, clipboardT, setting, value, option);
  });

  const aiTemplates = aiSelectedValues.map((id) => {
    return resolveAiTemplateLabel(id);
  });

  if (aiTemplates.length > 0) {
    labels.push(...aiTemplates);
  }

  if (labels.length > 0) return labels.join(" / ");

  return translatePreferenceSetting(t, setting, "title");
}

/**
 * 解析 AI 模板 id 对应的展示名；取自当前模板配置，找不到则回退为 id。
 */
function resolveAiTemplateLabel(id: string) {
  const template = settingsState.ai.customTemplates.find((tpl) => {
    return tpl.id === id;
  });

  return template?.name ?? id;
}

/**
 * 根据保存的完整顺序与 schema 默认顺序生成单列 Tree 数据；配置了 aiGroup 时
 * 在静态动作之后追加"AI 动作"分组（总闸关闭时禁用并提示）。
 */
function buildTreeData(
  t: TFunction<"preferences">,
  clipboardT: TFunction<"clipboard">,
  setting: PreferenceSetting,
  treeValue: { order: string[]; selected: string[] },
  ai: {
    aiEnabled: boolean;
    aiGroup?: { orderPath?: readonly string[]; path: readonly string[] };
    aiOrder: string[];
    aiTemplates: ReadonlyArray<{ id: string; name: string }>;
  },
) {
  if (setting.control.type !== "sortableCheckboxTree") return [];

  const options = setting.control.options;
  const optionValues = options.map((option) => {
    return String(option.value);
  });
  const orderedSet = new Set(treeValue.order);
  const orderedValues = [
    ...treeValue.order,
    ...optionValues.filter((value) => {
      return !orderedSet.has(value);
    }),
  ];

  const nodes = orderedValues.reduce<TreeDataNode[]>((acc, value) => {
    const option = options.find((item) => {
      return String(item.value) === value;
    });
    if (!option) return acc;

    const label = resolveOptionLabel(t, clipboardT, setting, value, option);

    acc.push({
      key: value,
      title: renderActionTitle(value, label),
    });

    return acc;
  }, []);

  if (!ai.aiGroup) return nodes;

  const groupTitle = t("schema.settings.aiGroup.title");
  const disabledHint = t("schema.settings.aiGroup.disabledHint");

  nodes.push({
    checkable: false,
    disabled: true,
    key: AI_GROUP_HEADER_KEY,
    selectable: false,
    title: ai.aiEnabled ? groupTitle : `${groupTitle}（${disabledHint}）`,
  });

  const orderSet = new Set(ai.aiOrder);
  const aiOrderedIds = [
    ...ai.aiOrder,
    ...ai.aiTemplates
      .map((tpl) => tpl.id)
      .filter((id) => {
        return !orderSet.has(id);
      }),
  ];

  for (const id of aiOrderedIds) {
    const template = ai.aiTemplates.find((tpl) => tpl.id === id);
    if (!template) continue;

    nodes.push({
      disabled: !ai.aiEnabled,
      key: id,
      title: (
        <span className="flex min-w-0 items-center gap-2">
          <i
            aria-hidden="true"
            className="i-lucide:sparkles shrink-0 text-ant-secondary"
          />
          <span className="min-w-0 truncate">{template.name}</span>
        </span>
      ),
    });
  }

  return nodes;
}

/**
 * 渲染动作项标题；图标放在 checkbox 后、文案前。
 */
function renderActionTitle(value: string, label: string) {
  const iconClass = isItemAction(value)
    ? resolveItemActionIcon(value)
    : "i-lucide:circle";

  return (
    <span className="flex min-w-0 items-center gap-2">
      <i
        aria-hidden="true"
        className={cn(iconClass, "shrink-0 text-ant-secondary")}
      />
      <span className="min-w-0 truncate">{label}</span>
    </span>
  );
}

/**
 * 解析树节点选项文案；快捷动作复用剪贴板窗口同一组文案。
 */
function resolveOptionLabel(
  t: TFunction<"preferences">,
  clipboardT: TFunction<"clipboard">,
  setting: PreferenceSetting,
  value: string,
  option: PreferenceOption,
) {
  if (isItemAction(value)) return translateItemActionLabel(clipboardT, value);

  return String(translatePreferenceOption(t, setting, option).label);
}
