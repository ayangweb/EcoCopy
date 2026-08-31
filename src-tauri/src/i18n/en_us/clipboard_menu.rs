use crate::i18n::keys::ClipboardMenuKey as Key;

/// 返回美式英文剪贴板列表项右键菜单文案。
pub fn label(key: Key) -> &'static str {
    match key {
        Key::Paste => "Paste",
        Key::PasteAsPlainText => "Paste as Plain Text",
        Key::PasteAsPath => "Paste as Path",
        Key::Copy => "Copy",
        Key::SaveImage => "Save Image",
        Key::OpenLink => "Open Link",
        Key::SendEmail => "Send Email",
        Key::RevealInFinder => "Show in Finder",
        Key::RevealInExplorer => "Show in Explorer",
        Key::Favorite => "Favorite",
        Key::Unfavorite => "Unfavorite",
        Key::PinItem => "Pin to Top",
        Key::UnpinItem => "Unpin from Top",
        Key::MoveToGroup => "Move to Group",
        Key::AddNote => "Add Note",
        Key::EditNote => "Edit Note",
        Key::Delete => "Delete",
        Key::AiProcess => "AI Process",
    }
}
