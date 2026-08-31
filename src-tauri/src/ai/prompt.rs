//! AI 单提示词构造。动作提示词与输入由程序按固定边界组合，用户不需要管理占位符。

use super::AiError;

/// 请求上下文：文本内容或图片附件由程序自动附加。
pub struct TemplateCtx {
    pub content: Option<String>,
    pub image: Option<String>,
}

/// 构造文本部分：动作提示词不能为空；文本动作必须提供内容。
/// 输入使用固定边界，避免剪贴板正文被模型误当成新的指令。
pub fn compose_prompt(prompt: &str, content: Option<&str>) -> Result<String, AiError> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(AiError::EmptyPrompt);
    }

    let Some(content) = content else {
        return Err(AiError::EmptyContent);
    };
    if content.trim().is_empty() {
        return Err(AiError::EmptyContent);
    }

    Ok(format!(
        "{prompt}\n\n--- 待处理内容开始 ---\n{content}\n--- 待处理内容结束 ---"
    ))
}

/// 构造图片动作的文字指令；图片附件由 provider 单独加入。
pub fn compose_image_prompt(prompt: &str) -> Result<String, AiError> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(AiError::EmptyPrompt);
    }

    Ok(prompt.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_text_prompt_adds_stable_content_boundary() {
        let out = compose_prompt("翻译为中文", Some("hello")).unwrap();

        assert_eq!(
            out,
            "翻译为中文\n\n--- 待处理内容开始 ---\nhello\n--- 待处理内容结束 ---"
        );
    }

    #[test]
    fn compose_text_prompt_rejects_empty_prompt_or_content() {
        assert!(matches!(
            compose_prompt(" ", Some("hello")),
            Err(AiError::EmptyPrompt)
        ));
        assert!(matches!(
            compose_prompt("翻译", Some(" ")),
            Err(AiError::EmptyContent)
        ));
        assert!(matches!(
            compose_prompt("翻译", None),
            Err(AiError::EmptyContent)
        ));
    }

    #[test]
    fn compose_image_prompt_does_not_need_content_placeholder() {
        assert_eq!(
            compose_image_prompt("识别图片文字").unwrap(),
            "识别图片文字"
        );
        assert!(matches!(
            compose_image_prompt(" "),
            Err(AiError::EmptyPrompt)
        ));
    }
}
