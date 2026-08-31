//! AI 命令入口：参数校验 + 启动/取消会话 + 连通性自检 + 结果写回剪贴板。业务在 ai/ 模块。

use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::ai::resolve_actions;
use crate::clipboard::{write_ai_text_to_clipboard, ImageStore, WritebackGuard};
use crate::core::{AppError, Result};
use crate::db::DatabaseState;
use crate::settings::SettingsStore;

/// run_ai_action 入参。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiActionInput {
    pub item_id: String,
    pub action_id: String,
}

/// check_ai_connectivity 入参：来自配置页当前输入（未保存也可点测）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAiConnectivityInput {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// 拉取当前可用的 AI 动作（未启用/未配置档案 → 空列表）。前端按条目 kind 过滤。
#[tauri::command]
pub async fn get_ai_actions(
    settings: State<'_, SettingsStore>,
) -> Result<Vec<crate::ai::AiActionInfo>> {
    let ai = settings.snapshot().ai.clone();

    if !ai.enabled || ai.models.is_empty() {
        return Ok(vec![]);
    }

    Ok(resolve_actions(&ai)
        .into_iter()
        .map(|a| crate::ai::AiActionInfo {
            id: a.id,
            input_kind: a.input_kind,
            label: a.label_key,
        })
        .collect())
}

/// 启动一个 AI 请求并立即返回 requestId。事件由任务自身 emit。
#[tauri::command]
pub async fn run_ai_action(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    image_store: State<'_, ImageStore>,
    sessions: State<'_, crate::ai::AiSessions>,
    settings: State<'_, SettingsStore>,
    input: RunAiActionInput,
) -> Result<String> {
    let pool = db.pool().await;
    let item = crate::db::items::find_item_by_id(&pool, &input.item_id)
        .await?
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("条目不存在")))?;

    let ai = settings.snapshot().ai.clone();
    if !ai.enabled || ai.models.is_empty() {
        return Err(AppError::Ai("请先完成 AI 配置".into()));
    }

    let action = resolve_actions(&ai)
        .into_iter()
        .find(|a| a.id == input.action_id)
        .ok_or_else(|| AppError::Ai(format!("未知动作: {}", input.action_id)))?;

    // 动作输入类型必须与条目类型一致：图片动作只能跑在图片条目上，反之亦然。
    // 不匹配时在派发前直接报错（toast），不进入结果弹窗。
    let kind_matches = match action.input_kind {
        crate::ai::AiInputKind::Image => item.kind == crate::db::models::ClipboardKind::Image,
        crate::ai::AiInputKind::Text => item.kind == crate::db::models::ClipboardKind::Text,
    };
    if !kind_matches {
        return Err(AppError::Ai(match action.input_kind {
            crate::ai::AiInputKind::Image => "该动作仅适用于图片条目".into(),
            crate::ai::AiInputKind::Text => "该动作仅适用于文本条目".into(),
        }));
    }

    // 模型档案：模板绑定优先，否则默认档案；两端点与模型名齐备才可用。
    let profile = action
        .model_profile_id
        .as_deref()
        .and_then(|id| ai.models.iter().find(|p| p.id == id))
        .or_else(|| ai.default_model())
        .filter(|profile| !profile.base_url.trim().is_empty() && !profile.model.trim().is_empty())
        .ok_or_else(|| AppError::Ai("请先完成 AI 配置".into()))?;

    let config = crate::ai::provider::ProviderConfig {
        base_url: profile.base_url.clone(),
        api_key: profile.api_key.clone(),
        model: profile.model.clone(),
    };

    let request_id = crate::ai::session::next_request_id();
    log::info!(
        "ai request {request_id} starting: action={} model={} streaming={} item_kind={:?}",
        action.id,
        profile.model,
        profile.streaming,
        item.kind,
    );
    let sessions_clone: crate::ai::AiSessions = (*sessions).clone();

    let app_for_task = app.clone();
    let item_for_task = item.clone();
    let action_for_task = action.clone();
    let config_for_task = config.clone();
    let image_store_for_task = image_store.inner().clone();
    let streaming = profile.streaming;
    let request_id_for_task = request_id.clone();

    let handle = tauri::async_runtime::spawn(async move {
        // ctx 构建放在任务内：构建失败同样 emit AI_ERROR（此时 reqId 已返回、
        // 弹窗已打开），错误必然可见，不会把弹窗留在"永远等待"。
        let ctx = match crate::ai::session::build_request_ctx(&item_for_task, &image_store_for_task)
            .await
        {
            Ok(ctx) => ctx,
            Err(err) => {
                log::warn!("ai request {request_id_for_task} failed: {err}");
                let _ = app_for_task.emit(
                    crate::ai::events::AI_ERROR,
                    serde_json::json!({
                        "requestId": request_id_for_task,
                        "message": err.to_string(),
                    }),
                );
                sessions_clone.remove(&request_id_for_task);
                return;
            }
        };

        let result = crate::ai::session::execute_ai_request(
            &app_for_task,
            streaming,
            &config_for_task,
            &action_for_task,
            &ctx,
            &request_id_for_task,
        )
        .await;

        if let Err(err) = result {
            log::warn!("ai request {request_id_for_task} failed: {err}");
            let _ = app_for_task.emit(
                crate::ai::events::AI_ERROR,
                serde_json::json!({
                    "requestId": request_id_for_task,
                    "message": err.to_string(),
                }),
            );
        }

        sessions_clone.remove(&request_id_for_task);
    });

    sessions.insert_with_id(&request_id, handle);
    Ok(request_id)
}

/// 取消正在运行的 AI 请求。
#[tauri::command]
pub async fn cancel_ai_request(
    sessions: State<'_, crate::ai::AiSessions>,
    request_id: String,
) -> Result<()> {
    let _ = sessions.cancel(&request_id);
    Ok(())
}

/// 连通性自检：发送极短请求验证配置可用。
#[tauri::command]
pub async fn check_ai_connectivity(input: CheckAiConnectivityInput) -> Result<String> {
    if input.base_url.trim().is_empty() || input.model.trim().is_empty() {
        return Err(AppError::Ai("请先填写 baseUrl 与 model".into()));
    }

    crate::ai::provider::check_connectivity(&crate::ai::provider::ProviderConfig {
        base_url: input.base_url,
        api_key: input.api_key,
        model: input.model,
    })
    .await
    .map_err(|e| AppError::Ai(e.to_string()))
}

/// 将 AI 结果文本写回系统剪贴板（不模拟粘贴）。
///
/// 与 `write_to_clipboard` 不同，此命令不依赖 DB 记录——AI 结果是新生成的文本，
/// 尚未入库。写回逻辑在 `clipboard::write::write_ai_text_to_clipboard` 内，
/// 包含回环抑制登记与写后回读校验。
#[tauri::command]
pub async fn write_ai_result_to_clipboard(
    guard: State<'_, Arc<WritebackGuard>>,
    text: String,
) -> Result<()> {
    write_ai_text_to_clipboard(guard.inner().as_ref(), &text)
}
