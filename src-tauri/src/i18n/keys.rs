#[derive(Debug, Clone, Copy)]
pub enum ClipboardMenuKey {
    Paste,
    PasteAsPlainText,
    PasteAsPath,
    Copy,
    SaveImage,
    OpenLink,
    SendEmail,
    RevealInFinder,
    RevealInExplorer,
    Favorite,
    Unfavorite,
    PinItem,
    UnpinItem,
    MoveToGroup,
    AddNote,
    EditNote,
    Delete,
    AiProcess,
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKey {
    DragSourceFilesMissing,
    DragImageMissing,
    DragTextEmpty,
    ExternalUrlUnsupported,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayKey {
    Preference,
    StartListening,
    StopListening,
    OpenSourceAddress,
    CheckForUpdates,
    Version,
    Relaunch,
    Exit,
}
