/**
 * AI 相关数据契约，镜像 Rust `src-tauri/src/ai/` 与 `src-tauri/src/commands/ai.rs`。
 */

/** `get_ai_actions` 返回的单条动作元信息。 */
export interface AiActionInfo {
  id: string;
  inputKind: "text" | "image";
  label: string;
}

/** `run_ai_action` 入参。 */
export interface RunAiActionInput {
  itemId: string;
  actionId: string;
}

/** `check_ai_connectivity` 入参。 */
export interface CheckAiConnectivityInput {
  baseUrl: string;
  apiKey: string;
  model: string;
}

/** `ai://chunk` payload。 */
export interface AiChunkPayload {
  requestId: string;
  delta: string;
}

/** `ai://done` payload。 */
export interface AiDonePayload {
  requestId: string;
  text: string;
}

/** `ai://error` payload。 */
export interface AiErrorPayload {
  requestId: string;
  message: string;
}
