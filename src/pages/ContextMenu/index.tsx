import { emitTo } from "@tauri-apps/api/event";
import { useEventListener, useMount } from "ahooks";
import {
  type FC,
  Fragment,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
  useState,
} from "react";
import {
  type ContextMenuShowPayload,
  type ContextSubmenuGroupInput,
  getContextMenuPayload,
  getContextSubmenuPayload,
  hideContextMenus,
  hideContextSubmenu,
  type ShowContextSubmenuInput,
  showContextSubmenu,
} from "@/commands";
import { TAURI_EVENT } from "@/constants/events";
import { WINDOW_LABEL } from "@/constants/windows";
import { useTauriListen } from "@/hooks/useTauriListen";
import type { ClipboardAction } from "@/types/clipboard";
import { cn } from "@/utils/cn";
import { formatShortcutDisplay } from "@/utils/shortcut";

interface MenuSurfaceProps {
  children: ReactNode;
}

interface BeforeDestroyPayload {
  label: string;
}

/**
 * Renders the menu surface flush to the transparent webview bounds.
 */
const MenuSurface: FC<MenuSurfaceProps> = (props) => {
  const { children } = props;

  return (
    <div className="fixed inset-0 box-border overflow-hidden">
      <div className="box-border h-full w-full select-none rounded-2 border border-ant-border-secondary bg-ant-elevated p-1 shadow-lg">
        {children}
      </div>
    </div>
  );
};

/**
 * Renders the root Windows custom context menu window.
 */
const ContextMenu: FC = () => {
  const [payload, setPayload] = useState<ContextMenuShowPayload | null>(null);
  const [activeSubmenuAction, setActiveSubmenuAction] =
    useState<ClipboardAction | null>(null);

  useTauriListen<ContextMenuShowPayload>(
    TAURI_EVENT.CONTEXT_MENU_SHOW,
    (event) => {
      setPayload(event.payload);
      setActiveSubmenuAction(null);
    },
  );

  const handleBeforeDestroy = (event: { payload: BeforeDestroyPayload }) => {
    if (event.payload.label !== WINDOW_LABEL.CONTEXT_MENU) return;

    setPayload(null);
    setActiveSubmenuAction(null);
  };

  useTauriListen<BeforeDestroyPayload>(
    TAURI_EVENT.WINDOW_BEFORE_DESTROY,
    handleBeforeDestroy,
  );

  useMount(async () => {
    const latestPayload = await getContextMenuPayload();
    if (!latestPayload) return;

    setPayload(latestPayload);
    setActiveSubmenuAction(null);
  });

  useEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;

    void hideContextMenus();
  });

  const handlePick = async (action: ClipboardAction, aiActionId?: string) => {
    if (!payload) return;

    await emitTo(WINDOW_LABEL.CLIPBOARD, TAURI_EVENT.CLIPBOARD_MENU_ACTION, {
      action,
      itemId: payload.itemId,
      ...(action === "aiProcess" && aiActionId ? { aiActionId } : {}),
    });

    await hideContextMenus();
  };

  const handlePickAi = async (aiActionId: string) => {
    if (!payload) return;

    await emitTo(WINDOW_LABEL.CLIPBOARD, TAURI_EVENT.CLIPBOARD_MENU_ACTION, {
      action: "aiProcess",
      aiActionId,
      itemId: payload.itemId,
    });

    await hideContextMenus();
  };

  const openSubmenu = (input: ShowContextSubmenuInput) => {
    setActiveSubmenuAction(input.action);
    void showContextSubmenu(input);
  };

  const closeSubmenu = () => {
    setActiveSubmenuAction(null);
    void hideContextSubmenu();
  };

  if (!payload) return null;

  return (
    <MenuSurface>
      {payload.groups.map((group, groupIndex) => {
        return (
          // biome-ignore lint/suspicious/noArrayIndexKey: backend-defined group order is stable.
          <Fragment key={groupIndex}>
            {groupIndex > 0 && <div className="my-1 h-px bg-ant-split" />}
            {group.map((item) => {
              return (
                <ContextMenuItem
                  accelerator={item.accelerator}
                  action={item.action}
                  aiActionId={item.aiActionId}
                  groups={item.groups ?? []}
                  isActive={activeSubmenuAction === item.action}
                  isDanger={item.action === "delete"}
                  itemId={payload.itemId}
                  key={item.aiActionId ?? item.action}
                  label={item.label}
                  onCloseSubmenu={closeSubmenu}
                  onOpenSubmenu={openSubmenu}
                  onPick={handlePick}
                  onPickAi={handlePickAi}
                />
              );
            })}
          </Fragment>
        );
      })}
    </MenuSurface>
  );
};

interface ContextMenuItemProps {
  accelerator: string | null;
  action: ClipboardAction;
  aiActionId?: string;
  groups: ContextSubmenuGroupInput[];
  isActive: boolean;
  isDanger: boolean;
  itemId: string;
  label: string;
  onCloseSubmenu: () => void;
  onOpenSubmenu: (input: ShowContextSubmenuInput) => void;
  onPick: (action: ClipboardAction, aiActionId?: string) => void;
  onPickAi: (aiActionId: string) => void;
}

/**
 * Renders a root menu item and asks Rust to show the submenu window when needed.
 */
const ContextMenuItem: FC<ContextMenuItemProps> = (props) => {
  const {
    accelerator,
    action,
    aiActionId,
    groups,
    isActive,
    isDanger,
    itemId,
    label,
    onCloseSubmenu,
    onOpenSubmenu,
    onPick,
    onPickAi,
  } = props;
  const hasGroups = groups.length > 0;

  const handleClick = () => {
    if (hasGroups) return;

    if (action === "aiProcess" && aiActionId) {
      onPickAi(aiActionId);

      return;
    }

    onPick(action);
  };

  const handlePointerEnter = (event: PointerEvent<HTMLButtonElement>) => {
    if (!hasGroups) {
      onCloseSubmenu();

      return;
    }

    const rect = event.currentTarget.getBoundingClientRect();
    onOpenSubmenu({
      action,
      anchor: {
        height: rect.height,
        left: rect.left,
        top: rect.top,
        width: rect.width,
      },
      groups,
      itemId,
    });
  };

  return (
    <button
      className={cn(
        "flex h-8 w-full cursor-pointer items-center justify-between rounded-1.5 border-0 bg-transparent px-3 text-sm transition-colors",
        {
          "bg-ant-text-hover": isActive && !isDanger,
          "hover:(text-ant-light-solid bg-ant-error) text-ant-error": isDanger,
          "hover:bg-ant-text-hover": !isDanger,
        },
      )}
      onClick={handleClick}
      onPointerEnter={handlePointerEnter}
      type="button"
    >
      <span className="truncate">{label}</span>
      {hasGroups ? (
        <span className="i-lucide:chevron-right ml-3 size-4 text-ant-description" />
      ) : (
        accelerator && (
          <span className="ml-3 whitespace-nowrap text-ant-description text-xs">
            {formatShortcutDisplay(accelerator)}
          </span>
        )
      )}
    </button>
  );
};

/**
 * Renders the secondary Windows custom context menu window.
 */
export const ContextSubmenu: FC = () => {
  const [payload, setPayload] = useState<ShowContextSubmenuInput | null>(null);

  useTauriListen<ShowContextSubmenuInput>(
    TAURI_EVENT.CONTEXT_SUBMENU_SHOW,
    (event) => {
      setPayload(event.payload);
    },
  );

  const handleBeforeDestroy = (event: { payload: BeforeDestroyPayload }) => {
    if (event.payload.label !== WINDOW_LABEL.CONTEXT_SUBMENU) return;

    setPayload(null);
  };

  useTauriListen<BeforeDestroyPayload>(
    TAURI_EVENT.WINDOW_BEFORE_DESTROY,
    handleBeforeDestroy,
  );

  useMount(async () => {
    const latestPayload = await getContextSubmenuPayload();
    if (!latestPayload) return;

    setPayload(latestPayload);
  });

  useEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;

    void hideContextMenus();
  });

  const handleGroupClick = async (event: MouseEvent<HTMLButtonElement>) => {
    if (!payload) return;

    const groupId = event.currentTarget.dataset.groupId;
    if (!groupId) return;

    if (payload.action === "aiProcess") {
      const aiActionId = groupId.replace("cim::ai::", "");

      await emitTo(WINDOW_LABEL.CLIPBOARD, TAURI_EVENT.CLIPBOARD_MENU_ACTION, {
        action: payload.action,
        aiActionId,
        itemId: payload.itemId,
      });
    } else {
      await emitTo(WINDOW_LABEL.CLIPBOARD, TAURI_EVENT.CLIPBOARD_MENU_ACTION, {
        action: payload.action,
        groupId,
        itemId: payload.itemId,
      });
    }

    await hideContextMenus();
  };

  if (!payload) return null;

  return (
    <MenuSurface>
      {payload.groups.map((group) => {
        return (
          <button
            className="flex h-8 w-full cursor-pointer items-center gap-2 rounded-1.5 border-0 bg-transparent px-3 text-left text-sm transition-colors hover:bg-ant-text-hover"
            data-group-id={group.id}
            key={group.id}
            onClick={handleGroupClick}
            type="button"
          >
            <span
              className={cn("size-4 text-ant-primary", {
                "i-lucide:check": group.checked,
              })}
            />
            <span className="truncate">{group.label}</span>
          </button>
        );
      })}
    </MenuSurface>
  );
};

export default ContextMenu;
