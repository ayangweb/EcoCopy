import { Button, Modal, Spin, Typography } from "antd";
import type { FC } from "react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSnapshot } from "valtio";
import { cancelAiRequest, writeAiResultToClipboard } from "@/commands";
import { TAURI_EVENT } from "@/constants/events";
import { useTauriListen } from "@/hooks/useTauriListen";
import { settingsState } from "@/stores/settings";
import type { AiChunkPayload, AiDonePayload, AiErrorPayload } from "@/types/ai";
import { getMessageApi } from "@/utils/feedback";
import { log } from "@/utils/log";

interface AiResultModalProps {
  requestId: string | null;
  onCancel: () => void;
}

const AiResultModal: FC<AiResultModalProps> = (props) => {
  const { requestId, onCancel } = props;
  const { t } = useTranslation("ai");
  const { ai } = useSnapshot(settingsState);
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const isAtBottomRef = useRef(true);

  useEffect(() => {
    setContent("");
    setLoading(Boolean(requestId));
    setError(null);
    isAtBottomRef.current = true;
  }, [requestId]);

  useTauriListen<AiChunkPayload>(TAURI_EVENT.AI_CHUNK, (event) => {
    if (event.payload.requestId !== requestId) return;

    setLoading(false);
    setContent((prev) => {
      const next = prev + event.payload.delta;
      queueMicrotask(() => {
        const body = bodyRef.current;
        if (body && isAtBottomRef.current) {
          body.scrollTop = body.scrollHeight;
        }
      });
      return next;
    });
  });

  useTauriListen<AiDonePayload>(TAURI_EVENT.AI_DONE, (event) => {
    if (event.payload.requestId !== requestId) return;

    setLoading(false);
    setContent(event.payload.text);

    queueMicrotask(() => {
      const body = bodyRef.current;
      if (body) {
        body.scrollTop = body.scrollHeight;
      }
    });

    if (!ai.autoWriteback) return;

    void (async () => {
      try {
        await writeAiResultToClipboard(event.payload.text);
        getMessageApi().success(t("writeback"));
      } catch (err) {
        log.error("auto writeback ai result failed", err);
      }
    })();
  });

  useTauriListen<AiErrorPayload>(TAURI_EVENT.AI_ERROR, (event) => {
    if (event.payload.requestId !== requestId) return;

    setLoading(false);
    setError(event.payload.message);
  });

  const handleBodyScroll = () => {
    const body = bodyRef.current;
    if (!body) return;

    isAtBottomRef.current =
      body.scrollHeight - body.scrollTop - body.clientHeight <= 24;
  };

  const handleCancel = async () => {
    if (requestId) {
      try {
        await cancelAiRequest(requestId);
      } catch (err) {
        log.error("cancel ai request failed", err);
      }
    }
    onCancel();
  };

  const writeResultToClipboard = async () => {
    if (!content) return;

    try {
      await writeAiResultToClipboard(content);
      getMessageApi().success(t("writeback"));
    } catch (err) {
      log.error("write ai result to clipboard failed", err);
    }
  };

  return (
    <Modal
      closable
      footer={(_, { CancelBtn }) => {
        return (
          <div className="flex items-center justify-end gap-2">
            <CancelBtn />
            <Button
              disabled={!content || loading || Boolean(error)}
              onClick={writeResultToClipboard}
              type="primary"
            >
              {t("writeback")}
            </Button>
          </div>
        );
      }}
      maskClosable={false}
      onCancel={handleCancel}
      open={Boolean(requestId)}
      title={t("resultTitle")}
      width={{ sm: 600, xs: "calc(100vw - 1rem)" }}
    >
      <div
        className="max-h-[calc(100dvh-12rem)] min-h-24 overflow-y-auto whitespace-pre-wrap break-words p-2 text-sm"
        onScroll={handleBodyScroll}
        ref={bodyRef}
      >
        {loading && (
          <div className="flex items-center gap-2 text-ant-secondary">
            <Spin size="small" />
            <span>{t("waiting")}</span>
          </div>
        )}

        {error && <Typography.Text type="danger">{error}</Typography.Text>}

        {content && !error && <Typography.Text>{content}</Typography.Text>}
      </div>
    </Modal>
  );
};

export default AiResultModal;
