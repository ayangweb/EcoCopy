//! AI 事件名常量。与 src/constants/events.ts 同步。

pub const AI_CHUNK: &str = "ai://chunk";
pub const AI_DONE: &str = "ai://done";
pub const AI_ERROR: &str = "ai://error";

#[cfg(test)]
mod tests {
    use super::*;

    /// 确保事件名遵循 `domain://action` 命名约定。
    #[test]
    fn event_names_follow_domain_action_convention() {
        assert!(AI_CHUNK.starts_with("ai://"));
        assert!(AI_DONE.starts_with("ai://"));
        assert!(AI_ERROR.starts_with("ai://"));
    }

    /// 确保三个事件名互不冲突。
    #[test]
    fn event_names_are_distinct() {
        let names = [AI_CHUNK, AI_DONE, AI_ERROR];
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(names[i], names[j]);
            }
        }
    }
}
