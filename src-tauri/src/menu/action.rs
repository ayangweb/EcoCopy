//! 右键菜单 AI 动作拆分逻辑。
//!
//! 与静态动作分组不同，AI 动作由设置里的 `aiVisible` / `aiOrder` 动态控制显示层级：
//! - 勾选提升的平铺为右键一级项
//! - 其余收进 AI 子菜单

use crate::menu::clipboard_item::AiMenuItem;

/// 按 `settings.menu.aiVisible/aiOrder` 把 AI 动作拆为两组：勾选提升为右键一级项的
/// （按配置顺序），与留在 AI 子菜单的其余动作。与 `request.ai_actions` 求交（已含
/// 启用/禁停过滤），模板删除后残留的 id 自然落空。
pub fn split_promoted_ai_actions(
    menu: &crate::settings::Menu,
    ai_actions: &[AiMenuItem],
) -> (Vec<AiMenuItem>, Vec<AiMenuItem>) {
    let promoted: Vec<AiMenuItem> = menu
        .ai_order
        .iter()
        .filter(|id| menu.ai_visible.contains(id))
        .filter_map(|id| ai_actions.iter().find(|action| &action.id == id))
        .cloned()
        .collect();

    let rest: Vec<AiMenuItem> = ai_actions
        .iter()
        .filter(|action| !promoted.iter().any(|p| p.id == action.id))
        .cloned()
        .collect();

    (promoted, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ai_item(id: &str) -> AiMenuItem {
        AiMenuItem {
            id: id.to_owned(),
            label: id.to_owned(),
            input_kind: crate::ai::AiInputKind::Text,
        }
    }

    #[test]
    fn split_promoted_ai_respects_visibility_and_order() {
        let menu = crate::settings::Menu {
            ai_visible: vec!["b".into(), "a".into()],
            ai_order: vec!["a".into(), "b".into(), "c".into()],
            ..Default::default()
        };
        let actions = vec![ai_item("a"), ai_item("b"), ai_item("c")];

        let (promoted, rest) = split_promoted_ai_actions(&menu, &actions);

        let promoted_ids: Vec<_> = promoted.iter().map(|a| a.id.as_str()).collect();
        let rest_ids: Vec<_> = rest.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(promoted_ids, ["a", "b"]);
        assert_eq!(rest_ids, ["c"]);
    }

    #[test]
    fn split_promoted_ai_defaults_to_all_in_submenu() {
        let menu = crate::settings::Menu::default();
        let actions = vec![ai_item("a"), ai_item("b")];

        let (promoted, rest) = split_promoted_ai_actions(&menu, &actions);

        assert!(promoted.is_empty());
        assert_eq!(rest.len(), 2);
    }

    #[test]
    fn split_promoted_ai_drops_unknown_ids() {
        let menu = crate::settings::Menu {
            ai_visible: vec!["a".into(), "gone".into()],
            ai_order: vec!["a".into(), "gone".into()],
            ..Default::default()
        };
        let actions = vec![ai_item("a")];

        let (promoted, rest) = split_promoted_ai_actions(&menu, &actions);

        assert_eq!(promoted.len(), 1);
        assert!(rest.is_empty());
    }
}
