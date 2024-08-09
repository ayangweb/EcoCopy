// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod locales;
mod plugins;

use core::{error::redirect_panic_to_log, info, tray};
use plugins::{
    backup, clipboard, fs_extra, locale, mouse, ocr, paste,
    window::{self, show_window, MAIN_WINDOW_LABEL, PREFERENCE_WINDOW_LABEL},
};
use std::env;
use tauri::{
    async_runtime, generate_context, generate_handler, Builder, Manager, SystemTray, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_theme::ThemePlugin;
use tauri_plugin_window_state::StateFlags;

pub const AUTO_LAUNCH_ARG: &str = "--auto-launch";

fn main() {
    redirect_panic_to_log();

    let mut ctx = generate_context!();

    let package_info = ctx.package_info();
    let app_name = &package_info.name;
    let app_version = &package_info.version;
    let tooltip = format!("{app_name} v{app_version}");

    Builder::default()
        .setup(|app| {
            // 获取命令行参数，检查是否有 `info` 或 `i` 参数
            match app.get_cli_matches() {
                Ok(matches) => {
                    if matches.args["info"].value.as_bool().expect("参数错误") {
                        info::print_system_info();
                        info::print_app_info(app.app_handle());

                        std::process::exit(0);
                    }
                }
                Err(_) => {}
            }

            // 判断是否为自动启动
            let args: Vec<String> = env::args().collect();
            if !args.contains(&AUTO_LAUNCH_ARG.to_string()) {
                let window = app.get_window(PREFERENCE_WINDOW_LABEL).unwrap();
                window.show().unwrap();
            }

            let window = app.get_window(MAIN_WINDOW_LABEL).unwrap();

            // 在开发环境中或者打包时加上 `--features=devtools` 启动时自动打开控制台：https://tauri.app/v1/guides/debugging/application/#opening-devtools-programmatically
            #[cfg(any(debug_assertions, feature = "devtools"))]
            window.open_devtools();

            #[cfg(target_os = "macos")]
            {
                // 隐藏 mac 下的程序坞图标：https://github.com/tauri-apps/tauri/issues/4852#issuecomment-1312716378
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                unsafe {
                    use cocoa::appkit::{NSMainMenuWindowLevel, NSWindow};
                    use cocoa::base::id;

                    let ns_window = window.ns_window().unwrap() as id;

                    // 让窗口在程序坞之上
                    ns_window.setLevel_(NSMainMenuWindowLevel as i64 + 1);
                }
            }

            core::app::observe_app();

            let _ = (app, window);

            Ok(())
        })
        // 主题插件：https://github.com/wyhaya/tauri-plugin-theme
        .plugin(ThemePlugin::init(ctx.config_mut()))
        // 确保在 windows 和 linux 上只有一个 app 实例在运行：https://github.com/tauri-apps/plugins-workspace/tree/v1/plugins/single-instance
        .plugin(tauri_plugin_single_instance::init(
            |app_handle, _argv, _cwd| {
                let window = app_handle.get_window(MAIN_WINDOW_LABEL).unwrap();

                async_runtime::block_on(async move {
                    show_window(window).await;
                });
            },
        ))
        // app 自启动：https://github.com/tauri-apps/tauri-plugin-autostart
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTO_LAUNCH_ARG]),
        ))
        // 数据库：https://github.com/tauri-apps/tauri-plugin-sql
        .plugin(tauri_plugin_sql::Builder::default().build())
        // 自定义的窗口管理插件
        .plugin(window::init())
        // 自定义的 fs_extra 插件
        .plugin(fs_extra::init())
        // 自定义剪贴板插件
        .plugin(clipboard::init())
        // 右键菜单插件：https://github.com/c2r0b/tauri-plugin-context-menu
        .plugin(tauri_plugin_context_menu::init())
        // 自定义鼠标相关的插件
        .plugin(mouse::init())
        // 自定义图片识别插件
        .plugin(ocr::init())
        // 自定义备份插件
        .plugin(backup::init())
        // 自定义语言相关的插件
        .plugin(locale::init()) // 系统托盘：https://tauri.app/v1/guides/features/system-tray
        // 自定义粘贴的插件
        .plugin(paste::init())
        // 日志插件：https://github.com/tauri-apps/tauri-plugin-log
        .plugin(tauri_plugin_log::Builder::default().build())
        // 记住窗口状态的插件：https://github.com/tauri-apps/plugins-workspace/tree/v1/plugins/window-state
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        // 系统托盘：https://tauri.app/v1/guides/features/system-tray
        .system_tray(SystemTray::new().with_tooltip(&tooltip))
        .on_system_tray_event(tray::Tray::handler)
        .invoke_handler(generate_handler![])
        // 让 app 保持在后台运行：https://tauri.app/v1/guides/features/system-tray/#preventing-the-app-from-closing
        .on_window_event(|event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .run(ctx)
        .expect("error while running tauri application");
}
