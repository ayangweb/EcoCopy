import { useRequest } from "ahooks";
import { useSnapshot } from "valtio";

import { getAiActions } from "@/commands";
import { settingsState } from "@/stores/settings";
import type { AiActionInfo } from "@/types/ai";

/**
 * 拉取当前可用的 AI 动作列表。未配置/未启用 → 空数组。
 * 动作列表在设置变更后自动刷新。
 */
export const useAiActions = () => {
  const { ai } = useSnapshot(settingsState);

  return useRequest(
    async (): Promise<AiActionInfo[]> => {
      if (!ai?.enabled || ai.models.length === 0) {
        return [];
      }

      return getAiActions();
    },
    {
      cacheKey: "ai-actions",
      // 模板增删/改名都会改变 id 或名称集合；不刷新的话右键与悬停入口会拿着
      // 已删除的旧 id 触发"未知动作"，或重命名后菜单仍显示旧名。
      refreshDeps: [
        ai?.enabled,
        ai?.models.length,
        ai?.customTemplates.map((tpl) => tpl.id).join(","),
        ai?.customTemplates.map((tpl) => tpl.name).join(","),
      ],
    },
  );
};
