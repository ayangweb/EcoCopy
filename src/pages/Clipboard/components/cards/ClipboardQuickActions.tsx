import { useUnmount } from "ahooks";
import { Dropdown } from "antd";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { FC, MouseEvent, SyntheticEvent } from "react";
import { useRef, useState } from "react";
import Tooltip from "@/components/Tooltip";
import {
  filterAvailableItemActions,
  type ItemActionLabels,
  isCopyItemAction,
  resolveItemActionPresentation,
} from "@/constants/itemActions";
import { aiActionMatchesItemKind } from "@/constants/menuActions";
import type { AiActionInfo } from "@/types/ai";
import type { ClipboardItem } from "@/types/clipboard";
import type { ItemAction } from "@/types/settings";
import { cn } from "@/utils/cn";

interface ClipboardQuickActionsProps {
  item: ClipboardItem;
  labels?: ItemActionLabels;
  onQuickAction?: (action: ItemAction) => Promise<void> | void;
  quickActions: ItemAction[];
  visible: boolean;
  aiActions?: AiActionInfo[];
  aiQuickActions?: readonly string[];
  onAiAction?: (actionId: string) => void;
}

interface QuickActionButtonProps {
  action: ItemAction;
  isFavorite: boolean;
  hasNote: boolean;
  isPinned: boolean;
  labels: ItemActionLabels;
  onQuickAction: (action: ItemAction) => Promise<void> | void;
  tabIndex: 0 | -1;
}

/**
 * 在卡片 meta 右侧展示时间，并在 hover 时替换为当前条目可执行的快捷动作。
 */
const ClipboardQuickActions: FC<ClipboardQuickActionsProps> = (props) => {
  const {
    item,
    labels,
    onQuickAction,
    quickActions,
    visible,
    aiActions,
    aiQuickActions,
    onAiAction,
  } = props;
  const shouldReduceMotion = useReducedMotion();
  const availableActions = filterAvailableItemActions(quickActions, item);
  const enabled =
    availableActions.length > 0 && Boolean(labels && onQuickAction);
  const actionsVisible = visible && enabled;
  const tabIndex = actionsVisible ? 0 : -1;
  const actionTransition = {
    duration: shouldReduceMotion ? 0 : 0.16,
    ease: "easeOut",
  } as const;

  // AI 快捷动作：按配置过滤，并按条目类型匹配。
  const aiCommon = (aiActions ?? [])
    .filter((a) => (aiQuickActions ?? []).includes(a.id))
    .filter((a) => aiActionMatchesItemKind(a.inputKind, item.kind));
  const hasAi = aiCommon.length > 0 && Boolean(onAiAction);
  const showMore = hasAi || availableActions.length > 4;

  return (
    <div className="grid h-6 shrink-0 items-center justify-items-end overflow-hidden">
      <span
        className={cn(
          "col-start-1 row-start-1 transition-all duration-150 ease-out motion-reduce:transition-none",
          {
            "-translate-x-1 opacity-0": actionsVisible,
          },
        )}
      >
        {item.displayCreatedAt ?? item.createdAt}
      </span>

      {enabled && onQuickAction && labels ? (
        <div
          aria-hidden={!actionsVisible}
          className={cn(
            "pointer-events-none col-start-1 row-start-1 flex translate-x-1 items-center gap-0.5 opacity-0 transition-all duration-150 ease-out motion-reduce:transition-none",
            {
              "pointer-events-auto translate-x-0 opacity-100": actionsVisible,
            },
          )}
        >
          <AnimatePresence initial={false} mode="popLayout">
            {availableActions.map((action) => {
              return (
                <motion.span
                  animate={{ opacity: 1, scale: 1, width: "1.25rem", x: 0 }}
                  className="flex overflow-hidden"
                  exit={{
                    opacity: 0,
                    scale: shouldReduceMotion ? 1 : 0.9,
                    width: 0,
                    x: shouldReduceMotion ? 0 : 4,
                  }}
                  initial={{ opacity: 0, scale: 0.9, width: 0, x: 4 }}
                  key={action}
                  layout
                  transition={actionTransition}
                >
                  <QuickActionButton
                    action={action}
                    hasNote={Boolean(item.note)}
                    isFavorite={item.isFavorite}
                    isPinned={item.isPinned}
                    labels={labels}
                    onQuickAction={onQuickAction}
                    tabIndex={tabIndex}
                  />
                </motion.span>
              );
            })}
          </AnimatePresence>
          {hasAi && onAiAction
            ? aiCommon.map((ai) => {
                return (
                  <motion.span
                    animate={{ opacity: 1, scale: 1, width: "1.25rem", x: 0 }}
                    className="flex overflow-hidden"
                    exit={{
                      opacity: 0,
                      scale: shouldReduceMotion ? 1 : 0.9,
                      width: 0,
                      x: shouldReduceMotion ? 0 : 4,
                    }}
                    initial={{ opacity: 0, scale: 0.9, width: 0, x: 4 }}
                    key={`ai-${ai.id}`}
                    layout
                    transition={actionTransition}
                  >
                    <Tooltip title={ai.label}>
                      <button
                        aria-label={ai.label}
                        className="flex size-5 items-center justify-center rounded-1.5 border-0 bg-transparent text-ant-secondary transition-colors hover:bg-ant-fill-tertiary hover:text-ant-text motion-reduce:transition-none"
                        onClick={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          onAiAction(ai.id);
                        }}
                        onPointerDown={(e) => e.stopPropagation()}
                        tabIndex={tabIndex}
                        type="button"
                      >
                        <i
                          aria-hidden="true"
                          className="i-lucide:sparkles text-sm"
                        />
                      </button>
                    </Tooltip>
                  </motion.span>
                );
              })
            : null}
          {showMore && labels ? (
            <Dropdown
              menu={{
                items: [
                  ...availableActions.slice(4).map((a) => ({
                    key: a,
                    label: labels[a],
                    onClick: () => onQuickAction(a),
                  })),
                  ...(hasAi && onAiAction
                    ? [
                        { type: "divider" as const },
                        ...aiCommon.map((a) => ({
                          key: `ai-${a.id}`,
                          label: a.label,
                          onClick: () => onAiAction(a.id),
                        })),
                      ]
                    : []),
                ],
              }}
              trigger={["click"]}
            >
              <button
                className="flex size-5 items-center justify-center rounded-1.5 border-0 bg-transparent text-ant-secondary hover:bg-ant-fill-tertiary hover:text-ant-text"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                }}
                onPointerDown={(e) => e.stopPropagation()}
                tabIndex={tabIndex}
                type="button"
              >
                <i aria-hidden="true" className="i-lucide:ellipsis text-sm" />
              </button>
            </Dropdown>
          ) : null}
        </div>
      ) : null}
    </div>
  );
};

export default ClipboardQuickActions;

/**
 * 单个 hover 快捷动作按钮；按下时阻止事件冒泡，避免触发卡片点击或自动粘贴。
 */
const QuickActionButton: FC<QuickActionButtonProps> = (props) => {
  const {
    action,
    hasNote,
    isFavorite,
    isPinned,
    labels,
    onQuickAction,
    tabIndex,
  } = props;
  const [copied, setCopied] = useState(false);
  const resetTimerRef = useRef<number | null>(null);
  const activeFavorite = action === "star" && isFavorite;
  const activeNote = action === "note" && hasNote;
  const activePinned = action === "pinItem" && isPinned;
  const activePrimary = activeNote || activePinned;
  const copyAction = isCopyItemAction(action);
  const presentation = resolveItemActionPresentation(action, labels, {
    copied,
    isFavorite: activeFavorite,
    isPinned: activePinned,
  });

  const stopQuickActionEvent = (event: SyntheticEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const clearCopiedResetTimer = () => {
    if (resetTimerRef.current === null) return;

    window.clearTimeout(resetTimerRef.current);
    resetTimerRef.current = null;
  };

  const startCopiedFeedback = () => {
    setCopied(true);
    clearCopiedResetTimer();
    resetTimerRef.current = window.setTimeout(() => {
      setCopied(false);
      resetTimerRef.current = null;
    }, 1000);
  };

  useUnmount(() => {
    clearCopiedResetTimer();
  });

  const handleClick = async (event: MouseEvent<HTMLButtonElement>) => {
    stopQuickActionEvent(event);

    if (copyAction && copied) return;

    await onQuickAction(action);

    if (!copyAction) return;

    startCopiedFeedback();
  };

  return (
    <Tooltip title={presentation.label}>
      <button
        aria-label={presentation.label}
        className={cn(
          "flex size-5 items-center justify-center rounded-1.5 border-0 bg-transparent text-ant-secondary transition-colors hover:bg-ant-fill-tertiary hover:text-ant-text motion-reduce:transition-none",
          {
            "text-ant-error hover:text-ant-error": presentation.danger,
            "text-ant-primary hover:text-ant-primary": activePrimary,
            "text-ant-success hover:text-ant-success": copied,
            "text-ant-warning hover:text-ant-warning": activeFavorite,
          },
        )}
        onAuxClick={stopQuickActionEvent}
        onClick={handleClick}
        onContextMenu={stopQuickActionEvent}
        onDoubleClick={stopQuickActionEvent}
        onMouseDown={stopQuickActionEvent}
        onPointerDown={stopQuickActionEvent}
        tabIndex={tabIndex}
        type="button"
      >
        <i aria-hidden="true" className={cn(presentation.icon, "text-sm")} />
      </button>
    </Tooltip>
  );
};
