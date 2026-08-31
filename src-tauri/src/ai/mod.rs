//! AI 辅助：动作注册表、模板渲染、OpenAI 兼容 provider 与请求会话管理。
//! 平台无关；Windows 主验证，macOS 编译保持通过。

pub mod actions;
pub mod events;
pub mod prompt;
pub mod provider;
pub mod session;

pub use actions::{resolve_actions, AiActionInfo, AiInputKind};
pub use prompt::TemplateCtx;
pub use session::AiSessions;

/// AI 模块统一错误。message 为用户可读根因，不加动作前缀。
#[derive(thiserror::Error, Debug)]
pub enum AiError {
    #[error("剪贴板条目内容为空或不可用")]
    EmptyContent,
    #[error("AI 动作提示词为空")]
    EmptyPrompt,
    #[error("图片文件不存在或超过大小上限")]
    ImageUnavailable,
    #[error("{0}")]
    Provider(#[from] anyhow::Error),
}
