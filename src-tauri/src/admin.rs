//! Windows administrator launch support.
//!
//! The persistent setting records intent. The current process token remains the
//! source of truth for whether the app is actually elevated.

#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use anyhow::Context;
#[cfg(target_os = "windows")]
use serde::Deserialize;

#[cfg(target_os = "windows")]
use crate::core::windows_args;
use crate::core::{AppError, Result};

#[cfg(target_os = "windows")]
const ADMIN_RESTARTED_ARG: &str = "--ecopaste-admin-restarted";
#[cfg(target_os = "windows")]
const TASK_NAME: &str = "EcoPasteAdmin";
#[cfg(target_os = "windows")]
const AUTOSTART_TASK_NAME: &str = "EcoPasteAdminAutostart";
#[cfg(target_os = "windows")]
const TASK_NAMES: [&str; 2] = [TASK_NAME, AUTOSTART_TASK_NAME];
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
const SETTINGS_FILENAME: &str = "settings.json";
#[cfg(target_os = "windows")]
const STORAGE_MANIFEST_FILENAME: &str = "storage.json";
#[cfg(target_os = "windows")]
const DEV_ENV_DIR: &str = "dev";
#[cfg(target_os = "windows")]
const PROD_ENV_DIR: &str = "prod";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLaunchStatus {
    pub configured: bool,
    pub running_as_admin: bool,
    pub task_ready: bool,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct EarlySettings {
    general: EarlyGeneral,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct EarlyGeneral {
    run_as_admin: bool,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StorageManifest {
    data_dir: PathBuf,
    environment: String,
    version: u16,
}

pub fn status(configured: bool) -> AdminLaunchStatus {
    AdminLaunchStatus {
        configured,
        running_as_admin: is_running_as_admin(),
        task_ready: is_scheduled_task_ready(),
    }
}

#[cfg(target_os = "windows")]
pub fn is_running_as_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0_u32;
        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        let _ = CloseHandle(token_handle);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_running_as_admin() -> bool {
    false
}

pub fn is_scheduled_task_ready() -> bool {
    #[cfg(target_os = "windows")]
    {
        TASK_NAMES.into_iter().all(|task_name| {
            is_scheduled_task_exists(task_name) && is_scheduled_task_path_valid(task_name)
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn sync_scheduled_task(configured: bool) {
    #[cfg(target_os = "windows")]
    {
        if !is_running_as_admin() {
            return;
        }

        for task_name in TASK_NAMES {
            let result = if configured {
                create_scheduled_task(task_name)
            } else {
                delete_scheduled_task(task_name)
            };
            if let Err(err) = result {
                log::warn!("sync admin scheduled task {task_name} failed: {err}");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = configured;
    }
}

pub fn launch_elevated_current_process() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        if try_launch_elevated_current_process() {
            return Ok(());
        }

        Err(AppError::Other(anyhow::anyhow!(
            "administrator permission request was cancelled or failed"
        )))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Other(anyhow::anyhow!(
            "administrator launch is only available on Windows"
        )))
    }
}

pub fn handle_startup_auto_elevation() {
    #[cfg(target_os = "windows")]
    {
        if cfg!(debug_assertions) {
            return;
        }

        let Ok(configured) = early_run_as_admin_enabled() else {
            return;
        };
        if !configured {
            return;
        }

        if is_running_as_admin() {
            sync_scheduled_task(true);
            return;
        }

        if has_admin_restart_marker() {
            return;
        }

        if try_launch_elevated_current_process() {
            std::process::exit(0);
        }
    }
}

#[cfg(target_os = "windows")]
fn is_scheduled_task_exists(task_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let output = Command::new("schtasks")
        .args(["/Query", "/TN", task_name])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    matches!(output, Ok(output) if output.status.success())
}

#[cfg(target_os = "windows")]
fn is_scheduled_task_path_valid(task_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let current_exe = match std::env::current_exe() {
        Ok(path) => path.to_string_lossy().to_lowercase(),
        Err(_) => return false,
    };
    let output = Command::new("schtasks")
        .args(["/Query", "/TN", task_name, "/FO", "LIST", "/V"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .to_lowercase()
        .contains(&current_exe)
}

#[cfg(target_os = "windows")]
fn create_scheduled_task(task_name: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let action = scheduled_task_action(&exe, task_name);

    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", task_name, "/TR", &action, "/SC", "ONCE", "/ST", "00:00", "/RL",
            "HIGHEST", "/F",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("failed to create administrator launch task")?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(AppError::Other(anyhow::anyhow!(
        "failed to create administrator launch task: {stderr}"
    )))
}

#[cfg(target_os = "windows")]
fn delete_scheduled_task(task_name: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("failed to delete administrator launch task")?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn run_via_scheduled_task(task_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let output = Command::new("schtasks")
        .args(["/Run", "/TN", task_name])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    matches!(output, Ok(output) if output.status.success())
}

#[cfg(target_os = "windows")]
fn try_launch_elevated_current_process() -> bool {
    if let Some(task_name) = scheduled_task_for_args(&std::env::args().skip(1).collect::<Vec<_>>())
    {
        if is_scheduled_task_exists(task_name)
            && is_scheduled_task_path_valid(task_name)
            && run_via_scheduled_task(task_name)
        {
            return true;
        }
    }

    try_launch_with_uac()
}

#[cfg(target_os = "windows")]
fn try_launch_with_uac() -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let params = restart_args()
        .into_iter()
        .map(|arg| windows_args::quote_arg(&arg))
        .collect::<Vec<_>>()
        .join(" ");

    let operation = wide_null("runas");
    let file: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params = wide_null(&params);

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(params.as_ptr()),
            PCWSTR(std::ptr::null()),
            SW_SHOWNORMAL,
        );

        result.0 as usize > 32
    }
}

#[cfg(target_os = "windows")]
fn early_run_as_admin_enabled() -> Result<bool> {
    let Some(base) = std::env::var_os("LOCALAPPDATA") else {
        return Ok(false);
    };

    let bootstrap = PathBuf::from(base)
        .join("com.ayangweb.eco-paste")
        .join(env_dir());
    let data_dir = early_data_dir(&bootstrap)?;
    let settings_path = data_dir.join("config").join(SETTINGS_FILENAME);
    if !settings_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("failed to read early settings at {settings_path:?}"))?;
    let settings: EarlySettings =
        serde_json::from_str(&content).context("failed to parse early settings")?;

    Ok(settings.general.run_as_admin)
}

#[cfg(target_os = "windows")]
fn early_data_dir(bootstrap: &Path) -> Result<PathBuf> {
    let default = bootstrap.to_path_buf();
    let manifest_path = bootstrap.join(STORAGE_MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Ok(default);
    }

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read storage manifest at {manifest_path:?}"))?;
    let manifest: StorageManifest =
        serde_json::from_str(&content).context("failed to parse storage manifest")?;
    if manifest.version != 1 || manifest.environment != env_dir() || !manifest.data_dir.exists() {
        return Ok(default);
    }

    Ok(manifest.data_dir)
}

#[cfg(target_os = "windows")]
/// Persistent actions depend only on task identity, never on the creating process's arguments.
fn scheduled_task_action(exe: &Path, task_name: &str) -> String {
    let mut action = format!(
        "{} {}",
        windows_args::quote_arg(exe.to_string_lossy()),
        windows_args::quote_arg(ADMIN_RESTARTED_ARG)
    );
    if task_name == AUTOSTART_TASK_NAME {
        action.push(' ');
        action.push_str(crate::autostart::AUTO_LAUNCH_ARG);
    }
    action
}

#[cfg(target_os = "windows")]
/// Fixed task actions cannot preserve file associations or arbitrary external arguments.
fn scheduled_task_for_args(args: &[String]) -> Option<&'static str> {
    if args
        .iter()
        .any(|arg| arg != ADMIN_RESTARTED_ARG && arg != crate::autostart::AUTO_LAUNCH_ARG)
    {
        return None;
    }

    Some(if crate::autostart::is_autostart_launch(args) {
        AUTOSTART_TASK_NAME
    } else {
        TASK_NAME
    })
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn routes_only_supported_arguments_to_the_matching_fixed_task() {
        let cases: &[(&[&str], Option<&str>)] = &[
            (&[], Some("EcoPasteAdmin")),
            (&[ADMIN_RESTARTED_ARG], Some("EcoPasteAdmin")),
            (&["--auto-launch"], Some("EcoPasteAdminAutostart")),
            (
                &[ADMIN_RESTARTED_ARG, "--auto-launch"],
                Some("EcoPasteAdminAutostart"),
            ),
            (
                &["--auto-launch", ADMIN_RESTARTED_ARG],
                Some("EcoPasteAdminAutostart"),
            ),
            (&[r"C:\Backups\history.ecopastebak"], None),
            (&["--auto-launch", r"C:\Backups\history.ecopastebak"], None),
            (
                &[ADMIN_RESTARTED_ARG, r"C:\Backups\history.ecopastebak"],
                None,
            ),
            (&["--unknown"], None),
            (&["--auto-launch", ADMIN_RESTARTED_ARG, "--unknown"], None),
        ];
        for (args, expected) in cases {
            let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
            assert_eq!(scheduled_task_for_args(&args), *expected, "{args:?}");
        }
    }

    #[test]
    fn scheduled_task_preserves_autostart_source() {
        assert_eq!(
            scheduled_task_action(Path::new(r"C:\Eco Paste\EcoPaste.exe"), AUTOSTART_TASK_NAME),
            r#""C:\Eco Paste\EcoPaste.exe" --ecopaste-admin-restarted --auto-launch"#
        );
    }

    #[test]
    fn scheduled_task_uses_internal_restart_source_for_manual_elevation() {
        assert_eq!(
            scheduled_task_action(Path::new(r"C:\Eco Paste\EcoPaste.exe"), TASK_NAME),
            r#""C:\Eco Paste\EcoPaste.exe" --ecopaste-admin-restarted"#
        );
    }
}

#[cfg(target_os = "windows")]
fn has_admin_restart_marker() -> bool {
    std::env::args().any(|arg| arg == ADMIN_RESTARTED_ARG)
}

#[cfg(target_os = "windows")]
fn restart_args() -> Vec<String> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|arg| arg != ADMIN_RESTARTED_ARG)
        .collect::<Vec<_>>();
    args.push(ADMIN_RESTARTED_ARG.to_owned());
    args
}

#[cfg(target_os = "windows")]
fn env_dir() -> &'static str {
    if cfg!(dev) {
        DEV_ENV_DIR
    } else {
        PROD_ENV_DIR
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
