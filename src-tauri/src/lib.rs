mod admin;
mod ai;
mod autostart;
mod backup;
mod clipboard;
mod commands;
mod core;
mod db;
mod drag_out;
mod i18n;
#[cfg(target_os = "windows")]
mod keyboard;
mod keystroke;
mod menu;
#[cfg(target_os = "windows")]
mod mouse;
mod settings;
mod shortcut;
mod tray;
mod update;
mod window;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    admin::handle_startup_auto_elevation();

    // Webview target 把日志回灌到前端 devtools console，只在 dev 启用；
    // 生产环境只落 LogDir 文件；Stdout 仅在 debug 启用，避免 Windows Release stdout 缓冲区阻塞，也避免向用户的 webview console 喷日志。
    let mut log_targets = vec![tauri_plugin_log::Target::new(
        tauri_plugin_log::TargetKind::LogDir { file_name: None },
    )];
    if cfg!(debug_assertions) {
        log_targets.push(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Stdout,
        ));
        log_targets.push(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Webview,
        ));
    }

    let log_plugin = tauri_plugin_log::Builder::new()
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .targets(log_targets)
        .build();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            |app_handle, argv, _cwd| {
                if let Some(path) = backup::backup_path_from_args(&argv) {
                    if let Err(err) = backup::emit_received_backup(
                        app_handle,
                        path,
                        backup::BackupReceiveSource::OpenFile,
                    ) {
                        log::error!("receive backup from second instance failed: {err:?}");
                    }
                    return;
                }

                // 登录时历史遗留的多个启动项可能并发拉起实例。第二实例只负责退出，
                // 不应按用户手动重复打开处理，否则会意外显示偏好设置窗口。
                if autostart::is_autostart_launch(&argv) {
                    return;
                }

                if let Err(err) = show_default_foreground_window(app_handle) {
                    log::error!("show foreground window on second instance failed: {err:?}");
                }
            },
        ))
        .plugin(log_plugin)
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_plugin_macos_permissions::init());

    let updater_plugin = match std::env::var("TAURI_UPDATER_PUBLIC_KEY")
        .or_else(|_| std::env::var("TAURI_SIGNING_PUBLIC_KEY"))
    {
        Ok(pubkey) if !pubkey.trim().is_empty() => {
            tauri_plugin_updater::Builder::new().pubkey(pubkey).build()
        }
        _ => tauri_plugin_updater::Builder::new().build(),
    };

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(updater_plugin)
        .plugin(core::prevent_default::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_run_as_admin_status,
            commands::set_run_as_admin,
            commands::restart_as_admin,
            commands::read_clipboard,
            commands::list_clipboard_items,
            commands::open_external_url,
            commands::list_clipboard_groups,
            commands::create_clipboard_group,
            commands::update_clipboard_group,
            commands::update_clipboard_groups_layout,
            commands::delete_clipboard_group,
            commands::import_clipboard_group_svg,
            commands::get_clipboard_item,
            commands::list_clipboard_apps,
            commands::list_all_apps,
            commands::add_clipboard_app_from_path,
            commands::delete_unreferenced_clipboard_apps,
            commands::get_clipboard_preview_payload,
            commands::play_copy_sound,
            commands::get_clipboard_image_path,
            commands::get_clipboard_app_icon_path,
            commands::save_clipboard_image_to_file,
            commands::get_file_icon_path,
            commands::write_to_clipboard,
            commands::paste_clipboard_item,
            commands::start_drag_clipboard_item,
            commands::toggle_clipboard_item_favorite,
            commands::toggle_clipboard_item_pinned,
            commands::delete_clipboard_item,
            commands::clear_clipboard_items,
            commands::update_clipboard_item_note,
            commands::update_clipboard_item_group,
            commands::open_clipboard_item_link,
            commands::reveal_clipboard_item,
            commands::show_window,
            commands::hide_window,
            commands::show_context_submenu,
            commands::hide_context_submenu,
            commands::hide_context_menus,
            commands::get_context_menu_payload,
            commands::get_context_submenu_payload,
            commands::toggle_window,
            commands::notify_window_ready,
            commands::set_window_dirty,
            commands::acquire_window_keepalive,
            commands::release_window_keepalive,
            commands::get_window_lifecycle_snapshot,
            commands::open_preference_with_highlight,
            commands::take_pending_preference_highlight,
            commands::open_update_window,
            commands::open_onboarding,
            commands::set_onboarding_step,
            commands::finish_onboarding,
            commands::detect_legacy_data,
            commands::import_legacy_data,
            commands::show_taskbar_icon,
            commands::position_window,
            commands::set_clipboard_window_pinned,
            commands::set_clipboard_window_auto_hide_suspended,
            commands::set_clipboard_window_editing,
            commands::show_clipboard_preview,
            commands::close_clipboard_preview,
            commands::get_clipboard_preview_state,
            commands::get_settings,
            commands::suspend_global_shortcuts,
            commands::resume_global_shortcuts,
            commands::update_settings,
            commands::reset_settings,
            commands::export_history_backup,
            commands::inspect_history_backup,
            commands::take_pending_backup,
            commands::import_history_backup,
            commands::get_storage_usage,
            commands::get_storage_location,
            commands::change_storage_location,
            commands::reset_storage_location,
            commands::clean_resource_cache,
            commands::open_preference_directory,
            commands::get_autostart,
            commands::set_autostart,
            commands::get_update_status,
            commands::check_for_updates,
            commands::download_update,
            commands::install_update,
            commands::skip_update_version,
            commands::get_ai_actions,
            commands::run_ai_action,
            commands::cancel_ai_request,
            commands::check_ai_connectivity,
            commands::write_ai_result_to_clipboard,
            menu::clipboard_item::popup_clipboard_item_menu,
        ])
        .on_menu_event(|app, event| {
            menu::clipboard_item::handle_menu_event(app, event.id().as_ref());
        })
        .setup(move |app| {
            let handle = app.handle().clone();

            // macOS：plugin 必须在 to_panel 前注册。
            #[cfg(target_os = "macos")]
            window::macos::register_plugin(&handle);

            let window_state_store = window::WindowStateStore::new(&handle).map_err(|err| {
                log::error!("window state store initialization failed: {err:?}");
                err
            })?;
            handle.manage(window_state_store);

            handle.manage(window::lifecycle::WindowLifecycleManager::new());
            update::init(&handle);

            let settings = settings::init(&handle).map_err(|err| {
                log::error!("settings initialization failed: {err:?}");
                err
            })?;

            let handle_db = handle.clone();
            tauri::async_runtime::block_on(async move {
                let pool = db::init(&handle_db).await.map_err(|err| {
                    log::error!("database initialization failed: {err:?}");
                    err
                })?;
                handle_db.manage(db::DatabaseState::new(pool));
                clipboard::init(&handle_db)?;
                Ok::<_, anyhow::Error>(())
            })?;

            handle.manage(ai::AiSessions::default());

            shortcut::init(&handle, &settings.shortcuts).map_err(|err| {
                log::error!("global shortcut initialization failed: {err:?}");
                err
            })?;

            autostart::init(&handle).map_err(|err| {
                log::error!("autostart initialization failed: {err:?}");
                err
            })?;
            if let Err(err) = autostart::sync_enabled(&handle, settings.general.auto_start) {
                log::warn!("autostart setting sync failed: {err}");
            }

            if let Err(err) = tray::init(&handle, &settings) {
                log::error!("tray initialization failed: {err:?}");
            }

            // 平台剪贴板窗口初始化：macOS 转 NSPanel
            #[cfg(target_os = "macos")]
            if let Err(err) = window::macos::setup_clipboard_panel(&handle) {
                log::error!("setup clipboard NSPanel failed: {err:?}");
            }

            menu::clipboard_item::init(&handle);

            #[cfg(target_os = "windows")]
            menu::context_window::init(&handle);

            if !settings.onboarding.completed {
                if let Err(err) = window::open_onboarding(&handle) {
                    log::error!("open onboarding window failed: {err:?}");
                }
            }

            update::schedule_auto_check(&handle);

            // Windows 冷启动文件关联：第一个实例从自身启动参数里取 `.ecopastebak` 路径。
            // 已运行时双击由 `single_instance` 回调处理；此处覆盖应用未启动时双击的冷启动场景，
            // 否则路径会被丢弃——程序被唤起但偏好窗口不弹。macOS 走 `RunEvent::Opened`，不经此路。
            #[cfg(target_os = "windows")]
            if let Some(path) = backup::backup_path_from_args(&std::env::args().collect::<Vec<_>>())
            {
                if let Err(err) = backup::emit_received_backup(
                    &handle,
                    path,
                    backup::BackupReceiveSource::OpenFile,
                ) {
                    log::error!("receive backup from launch args failed: {err:?}");
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window::intercept_close_request(window) {
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            // macOS 冷启动文件关联：`RunEvent::Ready` 早于系统投递的 `Opened`（多数情况），
            // 但二者顺序不保证。两端都经 backup 模块的就绪闸协调，谁先到都不会在未就绪时建窗。
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Ready = &event {
                backup::mark_app_ready(app_handle);
            }

            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                window::handle_reopen(app_handle, has_visible_windows);
            }

            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &event {
                // 处理体整体包 catch_unwind：本回调由 tao 从 ObjC `application:openURLs:`
                // 经 `extern "C"` 边界同步调用，任何 panic 都无法 unwind（`panic_cannot_unwind`）
                // 而直接 abort。即便就绪闸已规避主要 panic 源，这里仍兜底杜绝进程崩溃。
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    for url in urls {
                        if url.scheme() != "file" {
                            continue;
                        }
                        let Ok(path) = url.to_file_path() else {
                            continue;
                        };
                        if !backup::is_backup_path(&path) {
                            continue;
                        }
                        backup::handle_open_file(
                            app_handle,
                            path,
                            backup::BackupReceiveSource::OpenFile,
                        );
                        break;
                    }
                }));

                if result.is_err() {
                    log::error!("panic while handling Opened event (caught at C boundary)");
                }
            }

            // 退出前保存所有窗口几何，兜住「调整大小后不关窗直接退出」的场景。
            if let tauri::RunEvent::ExitRequested { .. } = event {
                window::save_all_window_states(app_handle);
            }
        });
}

fn show_default_foreground_window(app_handle: &tauri::AppHandle) -> core::Result<()> {
    if let Some(settings_store) = app_handle.try_state::<settings::SettingsStore>() {
        if !settings_store.snapshot().onboarding.completed {
            return window::open_onboarding(app_handle);
        }
    }

    window::show_window(app_handle, window::PREFERENCE_WINDOW_LABEL)
}
