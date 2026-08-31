//! `#[tauri::command]` 入口层（薄封装）：参数校验 + 调用 `clipboard` / `db` 下层，不写业务逻辑。

mod admin;
mod ai;
mod autostart;
mod backup;
mod clipboard;
mod context_menu;
mod drag;
mod link;
mod onboarding;
mod settings;
mod storage;
pub mod update;
mod window;

// glob 再导出：`#[tauri::command]` 会在函数旁生成隐藏辅助项（`__cmd__*`），
// `generate_handler!` 依赖它们，故用 `*` 一并带出，而非逐个具名 re-export。
pub use admin::*;
pub use ai::*;
pub use autostart::*;
pub use backup::*;
pub use clipboard::*;
pub use context_menu::*;
pub use drag::*;
pub use link::*;
pub use onboarding::*;
pub use settings::*;
pub use storage::*;
pub use update::*;
pub use window::*;
