use crate::i18n::keys::ClipboardMenuKey as Key;

/// 返回简体中文剪贴板列表项右键菜单文案。
pub fn label(key: Key) -> &'static str {
    match key {
        Key::Paste => "粘贴",
        Key::PasteAsPlainText => "粘贴为纯文本",
        Key::PasteAsPath => "粘贴为路径",
        Key::Copy => "复制",
        Key::SaveImage => "保存图片",
        Key::OpenLink => "打开链接",
        Key::SendEmail => "发送邮件",
        Key::RevealInFinder => "在访达中显示",
        Key::RevealInExplorer => "在资源管理器中显示",
        Key::Favorite => "收藏",
        Key::Unfavorite => "取消收藏",
        Key::PinItem => "置顶",
        Key::UnpinItem => "取消置顶",
        Key::MoveToGroup => "移动到分组",
        Key::AddNote => "添加备注",
        Key::EditNote => "编辑备注",
        Key::Delete => "删除",
        Key::AiProcess => "AI 处理",
    }
}
