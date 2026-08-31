//! AI 动作模板解析与过滤。

use serde::{Deserialize, Serialize};

use crate::settings::Ai;

/// 动作输入类型：文本 or 图片。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiInputKind {
    Text,
    Image,
}

/// 运行时动作描述（模板渲染后的统一形态）。
#[derive(Debug, Clone)]
pub struct AiAction {
    pub id: String,
    pub input_kind: AiInputKind,
    pub label_key: String,
    /// 用户配置的单一动作提示词；文本或图片由请求构造器自动附加。
    pub prompt: String,
    /// 本模板绑定的模型档案 id；空 = 跟随默认档案。
    pub model_profile_id: Option<String>,
}

/// 暴露给前端的动作信息（get_ai_actions 返回体）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiActionInfo {
    pub id: String,
    pub input_kind: AiInputKind,
    pub label: String,
}

/// 从 `custom_templates` 解析动作列表，再按 `disabled_actions` 过滤。
pub fn resolve_actions(ai: &Ai) -> Vec<AiAction> {
    let mut actions: Vec<AiAction> = ai
        .custom_templates
        .iter()
        .map(|tpl| AiAction {
            id: tpl.id.clone(),
            input_kind: tpl.input_kind,
            label_key: tpl.name.clone(),
            prompt: tpl.prompt.clone(),
            model_profile_id: tpl.model_profile_id.clone(),
        })
        .collect();

    actions.retain(|a| !ai.disabled_actions.contains(&a.id));

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AiActionTemplate;

    #[test]
    fn resolve_reads_custom_templates() {
        let ai = Ai {
            custom_templates: vec![
                AiActionTemplate {
                    id: "translate".into(),
                    name: "翻译".into(),
                    input_kind: AiInputKind::Text,
                    prompt: "sp".into(),
                    model_profile_id: None,
                },
                AiActionTemplate {
                    id: "ocr".into(),
                    name: "图片 OCR".into(),
                    input_kind: AiInputKind::Image,
                    prompt: "sp2".into(),
                    model_profile_id: None,
                },
            ],
            ..Default::default()
        };

        let acts = resolve_actions(&ai);
        assert_eq!(acts.len(), 2);
        assert_eq!(acts[0].id, "translate");
        assert_eq!(acts[1].id, "ocr");
    }

    #[test]
    fn resolve_filters_disabled() {
        let ai = Ai {
            custom_templates: vec![
                AiActionTemplate {
                    id: "a".into(),
                    name: "A".into(),
                    input_kind: AiInputKind::Text,
                    prompt: "sp".into(),
                    model_profile_id: None,
                },
                AiActionTemplate {
                    id: "b".into(),
                    name: "B".into(),
                    input_kind: AiInputKind::Text,
                    prompt: "sp".into(),
                    model_profile_id: None,
                },
            ],
            disabled_actions: vec!["a".into()],
            ..Default::default()
        };

        let acts = resolve_actions(&ai);
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].id, "b");
    }

    #[test]
    fn resolve_empty_when_no_templates() {
        let ai = Ai {
            custom_templates: vec![],
            ..Default::default()
        };

        assert!(resolve_actions(&ai).is_empty());
    }

    #[test]
    fn default_ai_has_four_templates() {
        let ai = Ai::default();
        let acts = resolve_actions(&ai);
        assert_eq!(acts.len(), 4);

        let text_count = acts
            .iter()
            .filter(|a| a.input_kind == AiInputKind::Text)
            .count();
        assert_eq!(text_count, 2);

        let image_count = acts
            .iter()
            .filter(|a| a.input_kind == AiInputKind::Image)
            .count();
        assert_eq!(image_count, 2);
    }
}
