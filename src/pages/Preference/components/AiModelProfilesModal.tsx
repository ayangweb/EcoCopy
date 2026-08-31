import type { TableColumnsType } from "antd";
import { Button, Form, Input, Modal, Space, Switch, Table, Tag } from "antd";
import type { FC } from "react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { checkAiConnectivity } from "@/commands";
import type { AiModelProfile } from "@/types/settings";
import { getMessageApi, getModalApi } from "@/utils/feedback";
import { log } from "@/utils/log";

interface AiModelProfilesModalProps {
  open: boolean;
  profiles: AiModelProfile[];
  defaultModelId: string | null;
  onCancel: () => void;
  onSave: (models: AiModelProfile[], defaultModelId: string | null) => void;
}

interface ProfileFormValues {
  name: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  streaming: boolean;
}

const AiModelProfilesModal: FC<AiModelProfilesModalProps> = (props) => {
  const { t } = useTranslation("ai");
  const { open, profiles, defaultModelId, onCancel, onSave } = props;
  const [items, setItems] = useState<AiModelProfile[]>([]);
  const [defaultId, setDefaultId] = useState<string | null>(null);
  const [editing, setEditing] = useState<AiModelProfile | null>(null);
  const [testing, setTesting] = useState(false);
  const [form] = Form.useForm<ProfileFormValues>();

  useEffect(() => {
    setItems(profiles);
    setDefaultId(defaultModelId);
  }, [profiles, defaultModelId]);

  const handleAdd = () => {
    setEditing({
      apiKey: "",
      baseUrl: "",
      id: `profile:${Date.now()}`,
      model: "",
      name: "",
      streaming: true,
    });
    form.resetFields();
  };

  const handleEdit = (record: AiModelProfile) => {
    setEditing(record);
    form.setFieldsValue({
      apiKey: record.apiKey,
      baseUrl: record.baseUrl,
      model: record.model,
      name: record.name,
      streaming: record.streaming,
    });
  };

  const handleDelete = (record: AiModelProfile) => {
    getModalApi().confirm({
      cancelText: t("common:actions.cancel"),
      content: t("modelProfiles.deleteConfirmContent", {
        name: record.name,
      }),
      okButtonProps: { danger: true },
      okText: t("common:actions.delete"),
      onOk: () => {
        setItems((prev) => prev.filter((item) => item.id !== record.id));
        setDefaultId((prev) => (prev !== record.id ? prev : null));
      },
      title: t("modelProfiles.deleteConfirmTitle"),
    });
  };

  const handleSetDefault = (record: AiModelProfile) => {
    setDefaultId(record.id);
  };

  const handleTest = async () => {
    try {
      const values = await form.validateFields();
      setTesting(true);
      await checkAiConnectivity({
        apiKey: values.apiKey,
        baseUrl: values.baseUrl,
        model: values.model,
      });
      getMessageApi().success(t("modelProfiles.testSuccess"));
    } catch (error) {
      log.warn("ai profile connectivity test failed", error);
      getMessageApi().error(t("modelProfiles.testFailure"));
    } finally {
      setTesting(false);
    }
  };

  const handleFormOk = async () => {
    let values: ProfileFormValues;

    try {
      values = await form.validateFields();
    } catch {
      return;
    }

    if (!editing) return;

    const profile: AiModelProfile = {
      apiKey: values.apiKey,
      baseUrl: values.baseUrl,
      id: editing.id,
      model: values.model,
      name: values.name,
      streaming: values.streaming,
    };

    setItems((prev) => {
      const idx = prev.findIndex((item) => item.id === profile.id);
      if (idx >= 0) {
        const next = [...prev];
        next[idx] = profile;
        return next;
      }
      return [...prev, profile];
    });
    setDefaultId((prev) => prev ?? profile.id);
    setEditing(null);
    form.resetFields();
  };

  const handleFormCancel = () => {
    setEditing(null);
    form.resetFields();
  };

  const handleSave = () => {
    onSave(items, defaultId);
  };

  const columns: TableColumnsType<AiModelProfile> = [
    {
      dataIndex: "name",
      key: "name",
      render: (value: string, record) => (
        <span className="inline-flex items-center gap-1.5">
          <span className="min-w-0 truncate">{value}</span>
          {record.id === defaultId ? (
            <Tag className="mr-0" color="blue">
              {t("modelProfiles.defaultBadge")}
            </Tag>
          ) : null}
        </span>
      ),
      title: t("modelProfiles.columns.name"),
    },
    {
      dataIndex: "baseUrl",
      ellipsis: true,
      key: "baseUrl",
      title: t("modelProfiles.columns.baseUrl"),
    },
    {
      dataIndex: "model",
      ellipsis: true,
      key: "model",
      title: t("modelProfiles.columns.model"),
    },
    {
      key: "actions",
      render: (_value, record) => (
        <Space>
          <Button onClick={() => handleEdit(record)} size="small">
            {t("common:actions.edit")}
          </Button>
          <Button
            disabled={record.id === defaultId}
            onClick={() => handleSetDefault(record)}
            size="small"
          >
            {t("modelProfiles.setDefault")}
          </Button>
          <Button danger onClick={() => handleDelete(record)} size="small">
            {t("common:actions.delete")}
          </Button>
        </Space>
      ),
      title: t("modelProfiles.columns.actions"),
      width: 220,
    },
  ];

  return (
    <Modal
      footer={
        <Space>
          <Button onClick={onCancel}>{t("common:actions.cancel")}</Button>
          <Button onClick={handleAdd}>{t("modelProfiles.add")}</Button>
          <Button onClick={handleSave} type="primary">
            {t("common:actions.save")}
          </Button>
        </Space>
      }
      onCancel={onCancel}
      open={open}
      title={t("modelProfiles.title")}
      width={720}
    >
      <Table
        columns={columns}
        dataSource={items}
        key="id"
        pagination={false}
        rowKey="id"
        size="small"
      />

      <Modal
        footer={
          <Space>
            <Button onClick={handleFormCancel}>
              {t("common:actions.cancel")}
            </Button>
            <Button loading={testing} onClick={handleTest}>
              {t("modelProfiles.test")}
            </Button>
            <Button onClick={handleFormOk} type="primary">
              {t("common:actions.save")}
            </Button>
          </Space>
        }
        onCancel={handleFormCancel}
        open={!!editing}
        title={editing ? t("modelProfiles.edit") : t("modelProfiles.add")}
      >
        <Form autoComplete="off" form={form} layout="vertical">
          <Form.Item
            label={t("modelProfiles.fields.name")}
            name="name"
            rules={[
              {
                message: t("modelProfiles.validation.nameRequired"),
                required: true,
              },
            ]}
          >
            <Input autoComplete="off" />
          </Form.Item>
          <Form.Item
            label={t("modelProfiles.fields.baseUrl")}
            name="baseUrl"
            rules={[
              {
                message: t("modelProfiles.validation.urlRequired"),
                required: true,
              },
            ]}
          >
            {/* WebView2 会无视表单级 off 对 URL 类字段弹「保存的信息」，
                非标准值才能稳定禁用自动填充。 */}
            <Input autoComplete="nope" />
          </Form.Item>
          <Form.Item label={t("modelProfiles.fields.apiKey")} name="apiKey">
            <Input.Password autoComplete="new-password" />
          </Form.Item>
          <Form.Item
            label={t("modelProfiles.fields.model")}
            name="model"
            rules={[
              {
                message: t("modelProfiles.validation.modelRequired"),
                required: true,
              },
            ]}
          >
            <Input autoComplete="nope" />
          </Form.Item>
          <Form.Item
            label={t("modelProfiles.fields.streaming")}
            name="streaming"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </Modal>
  );
};

export default AiModelProfilesModal;
