import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-dialog";
import type { TableColumnsType, TableProps } from "antd";
import { Button, Modal, Space, Table, Tooltip } from "antd";
import type { TFunction } from "i18next";
import type { FC } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  acquireWindowKeepalive,
  type ChangeStorageLocationResult,
  type CleanCacheResult,
  changeStorageLocation,
  cleanResourceCache,
  type ExportHistoryBackupResult,
  getWindowLifecycleSnapshot,
  inspectHistoryBackup,
  listClipboardGroups,
  openExternalUrl,
  openOnboarding,
  openPreferenceDirectory,
  openUpdateWindow,
  releaseWindowKeepalive,
  resetStorageLocation,
  type StorageLocation,
  type WindowLifecyclePhase,
  type WindowLifecycleSnapshot,
} from "@/commands";
import CustomIconButton from "@/components/CustomIconButton";
import { GITHUB_URL } from "@/constants/urls";
import { WINDOW_LABEL } from "@/constants/windows";
import {
  resetSettings,
  settingsState,
  updateSettings,
} from "@/stores/settings";
import type { ClipboardGroupRecord } from "@/types/clipboard";
import type { AiActionTemplate, AiModelProfile } from "@/types/settings";
import { getModalApi } from "@/utils/feedback";
import { log } from "@/utils/log";
import type { PreferenceSetting } from "../../types/preferences";
import { translatePreferenceControlLabel } from "../../utils/preferenceI18n";
import AiModelProfilesModal from "../AiModelProfilesModal";
import AiTemplateModal from "../AiTemplateModal";
import BackupExportModal from "../BackupExportModal";
import ClipboardGroupManagerModal from "../ClipboardGroupManagerModal";
import ControlFrame from "./ControlFrame";

const ABOUT_CHECK_UPDATES_SETTING_ID = "about.checkUpdates";
const ABOUT_GITHUB_SETTING_ID = "about.github";
const AI_MODELS_SETTING_ID = "ai.models";
const AI_TEMPLATES_SETTING_ID = "ai.customTemplates";
const BACKUP_EXTENSION = "ecopastebak";
const CLEAN_CACHE_SETTING_ID = "localData.cleanCache";
const CUSTOM_GROUPS_SETTING_ID = "organizing.customGroups";
const DATA_DIRECTORY_SETTING_ID = "localData.dataDirectory";
const EXPORT_BACKUP_SETTING_ID = "backup.exportHistory";
const IMPORT_BACKUP_SETTING_ID = "backup.importHistory";
const LOG_DIRECTORY_SETTING_ID = "localData.logDirectory";
const REOPEN_ONBOARDING_SETTING_ID = "control.reopenOnboarding";
const RESET_PREFERENCES_SETTING_ID = "diagnostics.resetPreferences";
const WINDOW_LIFECYCLE_SETTING_ID = "diagnostics.windowLifecycle";
const WINDOW_LIFECYCLE_I18N_PREFIX =
  "schema.settings.diagnostics.windowLifecycle";
const WINDOW_LIFECYCLE_DURATION_PART_LIMIT = 2;
const WINDOW_LIFECYCLE_DURATION_UNITS = [
  { key: "days", seconds: 86_400 },
  { key: "hours", seconds: 3_600 },
  { key: "minutes", seconds: 60 },
  { key: "seconds", seconds: 1 },
] as const;
const WINDOW_LIFECYCLE_PHASE_LABEL_KEYS: Record<WindowLifecyclePhase, string> =
  {
    created: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.created`,
    destroyed: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.destroyed`,
    destroyPending: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.destroyPending`,
    dormant: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.dormant`,
    hiddenWarm: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.hiddenWarm`,
    notCreated: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.notCreated`,
    ready: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.ready`,
    visible: `${WINDOW_LIFECYCLE_I18N_PREFIX}.phases.visible`,
  };
const WINDOW_LIFECYCLE_WINDOW_LABEL_KEYS: Record<string, string> = {
  [WINDOW_LABEL.CONTEXT_MENU]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.contextMenu`,
  [WINDOW_LABEL.CONTEXT_SUBMENU]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.contextSubmenu`,
  [WINDOW_LABEL.CLIPBOARD]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.clipboard`,
  [WINDOW_LABEL.ONBOARDING]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.onboarding`,
  [WINDOW_LABEL.PREFERENCE]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.preference`,
  [WINDOW_LABEL.PREVIEW]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.preview`,
  [WINDOW_LABEL.UPDATE]: `${WINDOW_LIFECYCLE_I18N_PREFIX}.windows.update`,
};

interface ActionControlProps {
  disabled: boolean;
  setting: PreferenceSetting;
  onActionComplete?: (
    setting: PreferenceSetting,
    result?:
      | ChangeStorageLocationResult
      | CleanCacheResult
      | ExportHistoryBackupResult,
  ) => void;
  storageLocation: StorageLocation | null;
}

/**
 * 展示右侧操作按钮，所有 action 控件保持同一尺寸。
 */
const ActionControl: FC<ActionControlProps> = (props) => {
  const { t } = useTranslation(["preferences", "common"]);
  const { disabled, setting, storageLocation, onActionComplete } = props;
  const [loading, setLoading] = useState(false);
  const [exportModalOpen, setExportModalOpen] = useState(false);
  const [groupManagerOpen, setGroupManagerOpen] = useState(false);
  const [groupManagerGroups, setGroupManagerGroups] = useState<
    ClipboardGroupRecord[]
  >([]);
  const [lifecycleModalOpen, setLifecycleModalOpen] = useState(false);
  const [lifecycleSnapshot, setLifecycleSnapshot] = useState<
    WindowLifecycleSnapshot[]
  >([]);
  const [modelsModalOpen, setModelsModalOpen] = useState(false);
  const [templatesModalOpen, setTemplatesModalOpen] = useState(false);
  const windowLabel = getCurrentWebviewWindow().label;

  if (setting.control.type !== "action") return null;

  /**
   * 操作进行中保活当前窗口；隐藏后 idle destroy 会等租约释放或超时兜底。
   */
  async function runWithKeepalive<T>(reason: string, task: () => Promise<T>) {
    const owner = `action:${setting.id}`;

    await acquireWindowKeepalive(windowLabel, owner, reason, 120_000);
    try {
      return await task();
    } finally {
      await releaseWindowKeepalive(windowLabel, owner);
    }
  }

  const markActionComplete = (
    result?:
      | ChangeStorageLocationResult
      | CleanCacheResult
      | ExportHistoryBackupResult,
  ) => {
    onActionComplete?.(setting, result);
  };

  const cleanCache = async () => {
    setLoading(true);
    try {
      const result = await runWithKeepalive("clean-cache", cleanResourceCache);
      markActionComplete(result);
    } finally {
      setLoading(false);
    }
  };

  const resetPreferenceSettings = async () => {
    setLoading(true);
    try {
      await runWithKeepalive("reset-preferences", resetSettings);
      markActionComplete();
    } finally {
      setLoading(false);
    }
  };

  const changeDataDirectory = async () => {
    setLoading(true);
    try {
      const selected = await runWithKeepalive("change-data-directory", () => {
        return open({
          directory: true,
          multiple: false,
          title: t("preferences:storageLocation.pickTitle"),
        });
      });
      if (!selected || Array.isArray(selected)) return;

      const result = await runWithKeepalive("change-data-directory", () => {
        return changeStorageLocation(selected);
      });
      markActionComplete(result);
    } finally {
      setLoading(false);
    }
  };

  const resetDataDirectory = async () => {
    setLoading(true);
    try {
      const result = await runWithKeepalive(
        "reset-data-directory",
        resetStorageLocation,
      );
      markActionComplete(result);
    } finally {
      setLoading(false);
    }
  };

  const confirmCleanCache = () => {
    getModalApi().confirm({
      cancelText: t("common:actions.cancel"),
      centered: true,
      content: t(
        "preferences:schema.settings.localData.cleanCache.confirmContent",
      ),
      okText: t(
        "preferences:schema.settings.localData.cleanCache.controlLabel",
      ),
      onOk: cleanCache,
      title: t("preferences:schema.settings.localData.cleanCache.confirmTitle"),
    });
  };

  const confirmResetPreferences = () => {
    getModalApi().confirm({
      cancelText: t("common:actions.cancel"),
      centered: true,
      content: t(
        "preferences:schema.settings.diagnostics.resetPreferences.confirmContent",
      ),
      okButtonProps: { danger: true },
      okText: t("common:actions.reset"),
      onOk: resetPreferenceSettings,
      title: t(
        "preferences:schema.settings.diagnostics.resetPreferences.confirmTitle",
      ),
    });
  };

  const confirmResetDataDirectory = () => {
    getModalApi().confirm({
      cancelText: t("common:actions.cancel"),
      centered: true,
      content: t("preferences:storageLocation.resetConfirmContent"),
      okText: t("preferences:storageLocation.reset"),
      onOk: resetDataDirectory,
      title: t("preferences:storageLocation.resetConfirmTitle"),
    });
  };

  const openExportModal = () => {
    setExportModalOpen(true);
  };

  const openGroupManager = async () => {
    setLoading(true);
    try {
      const groups = await runWithKeepalive(
        "open-group-manager",
        listClipboardGroups,
      );

      setGroupManagerGroups(groups);
      setGroupManagerOpen(true);
    } finally {
      setLoading(false);
    }
  };

  const openWindowLifecycleDiagnostics = async () => {
    setLoading(true);
    try {
      const snapshot = await getWindowLifecycleSnapshot();

      setLifecycleSnapshot(snapshot);
      setLifecycleModalOpen(true);
    } finally {
      setLoading(false);
    }
  };

  /**
   * 选择 `.ecopastebak` 文件并交给 Rust 识别，识别事件会打开统一导入弹窗。
   */
  const pickImportBackup = async () => {
    setLoading(true);
    try {
      const selected = await runWithKeepalive("pick-import-backup", () => {
        return open({
          directory: false,
          filters: [
            {
              extensions: [BACKUP_EXTENSION],
              name: t("preferences:backup.fileFilter"),
            },
          ],
          multiple: false,
          title: t("preferences:backup.import.pickTitle"),
        });
      });
      if (!selected || Array.isArray(selected)) return;

      await runWithKeepalive("inspect-import-backup", () => {
        return inspectHistoryBackup({
          path: selected,
          source: "openFile",
        });
      });
    } catch (error) {
      log.warn("pick backup import file failed", error);
    } finally {
      setLoading(false);
    }
  };

  const closeExportModal = () => {
    setExportModalOpen(false);
  };

  const closeGroupManager = () => {
    setGroupManagerOpen(false);
  };

  const closeLifecycleModal = () => {
    setLifecycleModalOpen(false);
  };

  const openModelsModal = () => {
    setModelsModalOpen(true);
  };

  const closeModelsModal = () => {
    setModelsModalOpen(false);
  };

  const handleModelsSaved = async (
    models: AiModelProfile[],
    defaultModelId: string | null,
  ) => {
    await updateSettings({
      ai: {
        defaultModelId: defaultModelId ?? undefined,
        models,
      },
    });
    setModelsModalOpen(false);
    markActionComplete();
  };

  const openTemplatesModal = () => {
    setTemplatesModalOpen(true);
  };

  const closeTemplatesModal = () => {
    setTemplatesModalOpen(false);
  };

  const handleTemplatesSaved = async (templates: AiActionTemplate[]) => {
    await updateSettings({
      ai: {
        customTemplates: templates,
      },
    });
    setTemplatesModalOpen(false);
    markActionComplete();
  };

  const handleBackupExported = (result: ExportHistoryBackupResult) => {
    setExportModalOpen(false);
    markActionComplete(result);
  };

  const handleGroupManagerSaved = () => {
    setGroupManagerOpen(false);
    markActionComplete();
  };

  const handleClick = async () => {
    if (setting.id === ABOUT_CHECK_UPDATES_SETTING_ID) {
      await runWithKeepalive("open-update-window", openUpdateWindow);
      markActionComplete();
      return;
    }

    if (setting.id === ABOUT_GITHUB_SETTING_ID) {
      await runWithKeepalive("open-github", () => {
        return openExternalUrl(GITHUB_URL);
      });
      markActionComplete();
      return;
    }

    if (setting.id === EXPORT_BACKUP_SETTING_ID) {
      openExportModal();
      return;
    }

    if (setting.id === IMPORT_BACKUP_SETTING_ID) {
      await pickImportBackup();
      return;
    }

    if (setting.id === CUSTOM_GROUPS_SETTING_ID) {
      await openGroupManager();
      return;
    }

    if (setting.id === DATA_DIRECTORY_SETTING_ID) {
      await openPreferenceDirectory("data");
      markActionComplete();
      return;
    }

    if (setting.id === REOPEN_ONBOARDING_SETTING_ID) {
      await runWithKeepalive("reopen-onboarding", openOnboarding);
      markActionComplete();
      return;
    }

    if (setting.id === LOG_DIRECTORY_SETTING_ID) {
      await runWithKeepalive("open-log-directory", () => {
        return openPreferenceDirectory("logs");
      });
      markActionComplete();
      return;
    }

    if (setting.id === AI_MODELS_SETTING_ID) {
      openModelsModal();
      return;
    }

    if (setting.id === AI_TEMPLATES_SETTING_ID) {
      openTemplatesModal();
      return;
    }

    if (setting.id === CLEAN_CACHE_SETTING_ID) {
      confirmCleanCache();
      return;
    }

    if (setting.id === RESET_PREFERENCES_SETTING_ID) {
      confirmResetPreferences();
      return;
    }

    if (setting.id === WINDOW_LIFECYCLE_SETTING_ID) {
      await openWindowLifecycleDiagnostics();
    }
  };

  const openDataDirectory = async () => {
    setLoading(true);
    try {
      await runWithKeepalive("open-data-directory", () => {
        return openPreferenceDirectory("data");
      });
      markActionComplete();
    } finally {
      setLoading(false);
    }
  };

  if (setting.id === DATA_DIRECTORY_SETTING_ID) {
    const canReset = storageLocation?.isCustom === true;

    return (
      <ControlFrame>
        <Space.Compact>
          <Tooltip title={t("preferences:storageLocation.open")}>
            <CustomIconButton
              disabled={disabled || loading}
              icon={<i aria-hidden="true" className="i-lucide:folder-open" />}
              loading={loading}
              onClick={openDataDirectory}
              type="default"
            />
          </Tooltip>

          <Tooltip title={t("preferences:storageLocation.change")}>
            <CustomIconButton
              disabled={disabled || loading}
              icon={<i aria-hidden="true" className="i-lucide:folder-sync" />}
              loading={loading}
              onClick={changeDataDirectory}
              type="default"
            />
          </Tooltip>

          <Tooltip title={t("preferences:storageLocation.reset")}>
            <CustomIconButton
              disabled={disabled || loading || !canReset}
              icon={<i aria-hidden="true" className="i-lucide:rotate-ccw" />}
              loading={loading}
              onClick={confirmResetDataDirectory}
              type="default"
            />
          </Tooltip>
        </Space.Compact>
      </ControlFrame>
    );
  }

  return (
    <>
      <ControlFrame>
        <Button
          danger={setting.control.danger}
          disabled={disabled || loading}
          loading={loading}
          onClick={handleClick}
        >
          {translatePreferenceControlLabel(t, setting)}
        </Button>
      </ControlFrame>

      {setting.id === EXPORT_BACKUP_SETTING_ID ? (
        <BackupExportModal
          onCancel={closeExportModal}
          onExported={handleBackupExported}
          open={exportModalOpen}
        />
      ) : null}

      {setting.id === CUSTOM_GROUPS_SETTING_ID ? (
        <ClipboardGroupManagerModal
          groups={groupManagerGroups}
          onCancel={closeGroupManager}
          onSaved={handleGroupManagerSaved}
          open={groupManagerOpen}
        />
      ) : null}

      {setting.id === WINDOW_LIFECYCLE_SETTING_ID ? (
        <Modal
          footer={null}
          onCancel={closeLifecycleModal}
          open={lifecycleModalOpen}
          title={t(
            "preferences:schema.settings.diagnostics.windowLifecycle.modalTitle",
          )}
        >
          <WindowLifecycleSnapshotTable rows={lifecycleSnapshot} />
        </Modal>
      ) : null}

      {setting.id === AI_MODELS_SETTING_ID ? (
        <AiModelProfilesModal
          defaultModelId={settingsState.ai.defaultModelId ?? null}
          onCancel={closeModelsModal}
          onSave={handleModelsSaved}
          open={modelsModalOpen}
          profiles={settingsState.ai.models}
        />
      ) : null}

      {setting.id === AI_TEMPLATES_SETTING_ID ? (
        <AiTemplateModal
          defaultModelId={settingsState.ai.defaultModelId ?? null}
          onCancel={closeTemplatesModal}
          onSave={handleTemplatesSaved}
          open={templatesModalOpen}
          profiles={settingsState.ai.models}
          templates={settingsState.ai.customTemplates}
        />
      ) : null}
    </>
  );
};

export default ActionControl;

interface WindowLifecycleSnapshotTableProps {
  rows: WindowLifecycleSnapshot[];
}

/**
 * 渲染窗口生命周期调试快照。
 */
const WindowLifecycleSnapshotTable: FC<WindowLifecycleSnapshotTableProps> = (
  props,
) => {
  const { t } = useTranslation("preferences");
  const { rows } = props;
  const columns: TableColumnsType<WindowLifecycleSnapshot> = [
    {
      dataIndex: "label",
      ellipsis: true,
      key: "label",
      render: (_value, row) => {
        return formatLifecycleWindowLabel(t, row.label);
      },
      title: t("schema.settings.diagnostics.windowLifecycle.columns.label"),
    },
    {
      dataIndex: "phase",
      ellipsis: true,
      key: "phase",
      render: (_value, row) => {
        return formatLifecyclePhase(t, row.phase);
      },
      title: t("schema.settings.diagnostics.windowLifecycle.columns.phase"),
    },
    {
      dataIndex: "generation",
      key: "generation",
      title: (
        <span className="inline-flex items-center gap-1 whitespace-nowrap">
          {t("schema.settings.diagnostics.windowLifecycle.columns.generation")}
          <Tooltip
            title={t(
              "schema.settings.diagnostics.windowLifecycle.generationHint",
            )}
          >
            <i
              aria-hidden="true"
              className="i-lucide:circle-help shrink-0 text-ant-tertiary"
            />
          </Tooltip>
        </span>
      ),
    },
    {
      key: "locks",
      render: (_value, row) => {
        return `${row.dirtyOwnerCount}/${row.keepaliveCount}`;
      },
      title: (
        <span className="inline-flex items-center gap-1 whitespace-nowrap">
          {t("schema.settings.diagnostics.windowLifecycle.columns.locks")}
          <Tooltip
            title={t("schema.settings.diagnostics.windowLifecycle.locksHint")}
          >
            <i
              aria-hidden="true"
              className="i-lucide:circle-help shrink-0 text-ant-tertiary"
            />
          </Tooltip>
        </span>
      ),
    },
    {
      ellipsis: true,
      key: "timing",
      render: (_value, row) => {
        return formatLifecycleTiming(t, row);
      },
      title: t("schema.settings.diagnostics.windowLifecycle.columns.timing"),
    },
  ];
  const tableLocale = {
    emptyText: t("schema.settings.diagnostics.windowLifecycle.empty"),
  } satisfies TableProps<WindowLifecycleSnapshot>["locale"];
  const tableScroll = {
    x: "max-content",
  } satisfies TableProps<WindowLifecycleSnapshot>["scroll"];

  return (
    <Table
      bordered
      columns={columns}
      dataSource={rows}
      locale={tableLocale}
      pagination={false}
      rowKey="label"
      scroll={tableScroll}
      size="small"
    />
  );
};

/**
 * 把内部窗口 label 转换为当前语言的诊断显示名；未知 label 保留原值便于排查。
 */
function formatLifecycleWindowLabel(
  t: TFunction<"preferences">,
  label: string,
) {
  const key = WINDOW_LIFECYCLE_WINDOW_LABEL_KEYS[label];
  if (!key) return label;

  return t(key);
}

/**
 * 把 Rust 生命周期阶段枚举转换为当前语言的诊断显示文案。
 */
function formatLifecyclePhase(
  t: TFunction<"preferences">,
  phase: WindowLifecyclePhase,
) {
  return t(WINDOW_LIFECYCLE_PHASE_LABEL_KEYS[phase]);
}

/**
 * 把毫秒时长压缩为最多两个单位的本地化短文本。
 */
function formatLifecycleDuration(
  t: TFunction<"preferences">,
  durationMs: number,
) {
  let remainingSeconds = Math.max(0, Math.round(durationMs / 1000));
  const parts: string[] = [];

  for (const unit of WINDOW_LIFECYCLE_DURATION_UNITS) {
    const value = Math.floor(remainingSeconds / unit.seconds);
    if (value === 0 && parts.length > 0) {
      continue;
    }

    if (value === 0 && unit.key !== "seconds") {
      continue;
    }

    parts.push(
      t(`${WINDOW_LIFECYCLE_I18N_PREFIX}.durationUnits.${unit.key}`, {
        count: value,
      }),
    );
    remainingSeconds -= value * unit.seconds;

    if (parts.length >= WINDOW_LIFECYCLE_DURATION_PART_LIMIT) break;
  }

  return parts.join(t(`${WINDOW_LIFECYCLE_I18N_PREFIX}.durationSeparator`));
}

/**
 * 格式化生命周期调试时间信息。
 */
function formatLifecycleTiming(
  t: TFunction<"preferences">,
  row: WindowLifecycleSnapshot,
) {
  if (row.hiddenForMs !== null) {
    return t("schema.settings.diagnostics.windowLifecycle.hiddenFor", {
      duration: formatLifecycleDuration(t, row.hiddenForMs),
    });
  }

  return t("schema.settings.diagnostics.windowLifecycle.lastActive", {
    duration: formatLifecycleDuration(t, row.lastActiveAgoMs),
  });
}
