import type { TableColumnsType } from "antd";
import { Button, Form, Input, Modal, Radio, Select, Space, Table } from "antd";
import type { FC } from "react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  AiActionTemplate,
  AiInputKind,
  AiModelProfile,
} from "@/types/settings";

interface AiTemplateModalProps {
  open: boolean;
  templates: AiActionTemplate[];
  profiles: AiModelProfile[];
  defaultModelId: string | null;
  onCancel: () => void;
  onSave: (templates: AiActionTemplate[]) => void;
}

interface TemplateFormValues {
  name: string;
  inputKind: "text" | "image";
  modelProfileId: string;
  prompt: string;
}

const AiTemplateModal: FC<AiTemplateModalProps> = (props) => {
  const { t } = useTranslation("ai");
  const { open, templates, profiles, defaultModelId, onCancel, onSave } = props;
  const [items, setItems] = useState<AiActionTemplate[]>([]);
  const [editing, setEditing] = useState<AiActionTemplate | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [form] = Form.useForm<TemplateFormValues>();

  useEffect(() => {
    setItems(templates);
  }, [templates]);

  const handleAdd = () => {
    setIsNew(true);
    setEditing({
      id: `custom:${Date.now()}`,
      inputKind: "text",
      name: "",
      prompt: "",
    });
    form.resetFields();
  };

  const handleEdit = (record: AiActionTemplate) => {
    setIsNew(false);
    setEditing(record);
    form.setFieldsValue({
      inputKind: record.inputKind,
      modelProfileId: record.modelProfileId ?? "",
      name: record.name,
      prompt: record.prompt,
    });
  };

  const handleDelete = (record: AiActionTemplate) => {
    setItems((prev) => prev.filter((item) => item.id !== record.id));
  };

  const handleFormOk = async () => {
    let values: TemplateFormValues;

    try {
      values = await form.validateFields();
    } catch {
      return;
    }

    const template: AiActionTemplate = {
      id: editing?.id || `custom:${Date.now()}`,
      inputKind: values.inputKind,
      modelProfileId: values.modelProfileId || undefined,
      name: values.name,
      prompt: values.prompt,
    };

    setItems((prev) => {
      const index = prev.findIndex((item) => item.id === template.id);
      if (index < 0) return [...prev, template];

      const next = [...prev];
      next[index] = template;
      return next;
    });

    closeForm();
  };

  const closeForm = () => {
    setEditing(null);
    form.resetFields();
  };

  const profileOptions = [
    {
      label: t("templateModal.form.followDefault", {
        name: resolveDefaultProfileName(profiles, defaultModelId),
      }),
      value: "",
    },
    ...profiles.map((profile) => {
      return { label: profile.name, value: profile.id };
    }),
  ];
  const inputKindOptions = (["text", "image"] as const).map((kind) => {
    return { label: t(`templateModal.inputKind.${kind}`), value: kind };
  });

  const columns: TableColumnsType<AiActionTemplate> = [
    {
      dataIndex: "name",
      key: "name",
      title: t("templateModal.columns.name"),
    },
    {
      dataIndex: "inputKind",
      key: "inputKind",
      render: (value: AiInputKind) => {
        return t(`templateModal.inputKind.${value}`);
      },
      title: t("templateModal.columns.inputKind"),
      width: 80,
    },
    {
      dataIndex: "modelProfileId",
      key: "modelProfileId",
      render: (value: string | null) => {
        if (!value) return t("templateModal.form.followDefaultShort");

        return profiles.find((profile) => profile.id === value)?.name ?? value;
      },
      title: t("templateModal.columns.model"),
      width: 140,
    },
    {
      key: "actions",
      render: (_value, record) => {
        return (
          <Space>
            <Button onClick={() => handleEdit(record)} size="small">
              {t("common:actions.edit")}
            </Button>
            <Button danger onClick={() => handleDelete(record)} size="small">
              {t("common:actions.delete")}
            </Button>
          </Space>
        );
      },
      title: t("templateModal.columns.actions"),
      width: 140,
    },
  ];

  return (
    <Modal
      footer={
        <Space>
          <Button onClick={onCancel}>{t("common:actions.cancel")}</Button>
          <Button onClick={handleAdd} type="dashed">
            {t("templateModal.add")}
          </Button>
          <Button onClick={() => onSave(items)} type="primary">
            {t("common:actions.save")}
          </Button>
        </Space>
      }
      onCancel={onCancel}
      open={open}
      title={t("templateModal.title")}
      width={640}
    >
      <Table
        columns={columns}
        dataSource={items}
        pagination={false}
        rowKey="id"
        size="small"
      />

      <Modal
        footer={
          <Space>
            <Button onClick={closeForm}>{t("common:actions.cancel")}</Button>
            <Button onClick={handleFormOk} type="primary">
              {t("common:actions.ok")}
            </Button>
          </Space>
        }
        onCancel={closeForm}
        open={editing !== null}
        title={isNew ? t("templateModal.add") : t("templateModal.edit")}
      >
        <Form
          form={form}
          initialValues={{
            inputKind: "text",
            modelProfileId: "",
          }}
          layout="vertical"
        >
          <Form.Item
            label={t("templateModal.form.name")}
            name="name"
            rules={[{ required: true }]}
          >
            <Input />
          </Form.Item>

          <Form.Item
            label={t("templateModal.form.inputKind")}
            name="inputKind"
            rules={[{ required: true }]}
          >
            <Radio.Group options={inputKindOptions} />
          </Form.Item>

          <Form.Item
            label={t("templateModal.form.model")}
            name="modelProfileId"
          >
            <Select options={profileOptions} />
          </Form.Item>

          <Form.Item
            extra={t("templateModal.form.promptHint")}
            label={t("templateModal.form.prompt")}
            name="prompt"
            rules={[{ required: true }]}
          >
            <Input.TextArea rows={6} />
          </Form.Item>
        </Form>
      </Modal>
    </Modal>
  );
};

function resolveDefaultProfileName(
  profiles: AiModelProfile[],
  defaultModelId: string | null,
) {
  const profile = defaultModelId
    ? profiles.find((item) => item.id === defaultModelId)
    : undefined;

  return profile?.name ?? profiles[0]?.name ?? "";
}

export default AiTemplateModal;
