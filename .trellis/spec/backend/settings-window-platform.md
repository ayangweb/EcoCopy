# Settings, Windowing, and Platform

## Settings Model

Settings live in Rust under `src-tauri/src/settings/`.

- `model.rs` defines the serialized shape, defaults, enums, and helper methods.
- Every settings struct uses `#[serde(default, rename_all = "camelCase")]`, so
  newly added fields fall back to defaults.
- `store.rs` persists `<app_data_dir>/config/settings.json` with a temp file and
  atomic rename.
- `SettingsStore::update` accepts only object patches, deep-merges objects,
  replaces arrays wholesale, validates cross-field constraints, writes to disk,
  and updates the in-memory snapshot.

When adding a setting, update:

- Rust model and defaults.
- Validation if the field interacts with another field.
- TypeScript `src/types/settings.ts`.
- Preference schema in `src/pages/Preference/config/preferenceSchema.ts` if the
  setting is user-visible.
- zh-CN and en-US locale entries.
- Side effects in `commands/settings.rs` or
  `src/pages/Preference/services/preferenceSettings.ts` when OS state changes
  immediately.

### Window Open Selection Settings

#### 1. Scope / Trigger

Changing the default filter applied when the clipboard window opens is a
settings contract change. Rust owns the persisted shape; React only mirrors it
and applies the transient `clipboardViewState` side effect on `window://visibility`.

#### 2. Signatures

- Rust fields:
  - `Window.select_range_on_open: WindowOpenRangeSelection`
  - `Window.select_category_on_open: WindowOpenCategorySelection`
  - `Window.select_group_on_open: String`
- TypeScript mirror:
  - `Window.selectRangeOnOpen: "preserve" | ClipboardRange`
  - `Window.selectCategoryOnOpen: "preserve" | "all" | ClipboardCategory`
  - `Window.selectGroupOnOpen: "preserve" | "all" | \`group:${string}\``

#### 3. Contracts

- `preserve` means keep the user's last selected value for that dimension.
- `all` means clear that dimension to the built-in All state.
- `group:<id>` means select the custom clipboard group with that id.
- Range options are `preserve`, `all`, and `favorite`.
- Category options are `preserve`, `all`, `text`, `image`, and `files`.

#### 4. Validation & Error Matrix

- Invalid range/category literals fail serde deserialization.
- `select_group_on_open = "preserve"` -> valid.
- `select_group_on_open = "all"` -> valid.
- `select_group_on_open = "group:<non-empty id>"` -> valid.
- Any other group string, including `group:` -> `AppError::Other("open group selection is invalid")`.

#### 5. Good/Base/Bad Cases

- Good: selecting `favorite`, `image`, and `group:<uuid>` opens into favorites,
  image category, and that custom group.
- Base: all three fields default to `preserve`, so opening the window keeps the
  user's last filters.
- Bad: storing a raw group UUID without the `group:` prefix is rejected.

#### 6. Tests Required

- Missing-field defaults assert all three fields default to `preserve`.
- Settings validation rejects invalid `select_group_on_open` strings.
- Frontend type-check covers the mirrored selection unions and the dynamic group
  Select value shape.

#### 7. Wrong vs Correct

Wrong:

```json
{ "clipboard": { "window": { "selectGroupOnOpen": "550e8400-e29b" } } }
```

Correct:

```json
{ "clipboard": { "window": { "selectGroupOnOpen": "group:550e8400-e29b" } } }
```

## Settings Events and Mirrors

Rust is the source of truth. `commands::settings::update_settings` and
`reset_settings` emit `settings://updated` after persistence and side effects.
`src/stores/settings.ts` listens to that event and updates the Valtio mirror.

Do not mutate `settingsState` directly from UI components. Even the initiating
window waits for the broadcast so all windows share the same snapshot.

## Window Labels and Lifecycle

Window labels are constants in `window/mod.rs` and mirrored in
`src/constants/windows.ts`. Current labels are:

- `clipboard`
- `preference`
- `clipboard-preview`
- `onboarding`
- `update`
- `context-menu`
- `context-submenu`

When a known window label is displayed in diagnostics UI, map it through
`src/locales/*/preferences.json` instead of rendering the raw internal label.
Keep raw-label fallback only for unknown or unregistered labels so diagnostics
remain useful.

Use `window::show_window`, `hide_window`, and `toggle_window` instead of direct
webview calls. These functions restore/save geometry, apply clipboard-window
position settings, emit `window://visibility`, update lifecycle state, and
handle platform-specific show/hide.

`window/lifecycle/mod.rs` tracks phases such as `ready`, `visible`,
`hiddenWarm`, `dormant`, `destroyPending`, and `destroyed`. Destroyable windows
use descriptors and can rebuild after idle destruction. Frontend windows report
readiness through `notify_window_ready` once `settingsReady` has resolved.

### Scenario: Clipboard Window Label Contract

#### 1. Scope / Trigger

Changing the primary clipboard-history window label touches Tauri config, Rust
window constants, frontend constants, commands, events, lifecycle diagnostics,
and preview payloads. Treat it as a cross-layer contract change.

#### 2. Signatures

- Runtime label:
  - Rust `window::CLIPBOARD_WINDOW_LABEL = "clipboard"`
  - TypeScript `WINDOW_LABEL.CLIPBOARD = "clipboard"`
  - Tauri config window label `"clipboard"`
- Commands:
  - `set_clipboard_window_pinned(pinned: bool) -> ()`
  - `set_clipboard_window_auto_hide_suspended(suspended: bool) -> ()`
- Preview payload:
  - `ClipboardPreviewState.clipboardWindow: PreviewClipboardWindowRect | null`

#### 3. Contracts

- User-visible copy should say clipboard window, not main window.
- User-visible names for concrete window entities should include the window
  suffix, for example `剪贴板窗口` / `Clipboard Window` and
  `偏好设置窗口` / `Preferences Window`.
- Do not use raw `"clipboard"` in React components; compare with
  `WINDOW_LABEL.CLIPBOARD`.
- Keep platform terms such as Rust `main()`, `run_on_main_thread`, and Cocoa
  `can_become_main_window` unchanged.

#### 4. Validation & Error Matrix

- Stale `"main"` label in a consumer -> lifecycle or emit target is ignored.
- Stale command name -> frontend invoke fails with an unknown command.
- Stale preview field name -> preview layout loses clipboard-window geometry.

#### 5. Good/Base/Bad Cases

- Good: tray, shortcut, onboarding, context menu, preview, and copy-after-hide
  all target `CLIPBOARD_WINDOW_LABEL` / `WINDOW_LABEL.CLIPBOARD`.
- Base: secondary windows still use their existing labels and lifecycle rules.
- Bad: component code checks `event.payload.label === "main"` directly.

#### 6. Tests Required

- `rg` verifies no stale `MAIN_WINDOW_LABEL`, `WINDOW_LABEL.MAIN`, raw
  `"main"` label, or old command names remain outside platform terminology.
- Frontend: `pnpm lint` and `pnpm tsc`.
- Backend: `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
- Manual desktop checks for clipboard show/hide/toggle, pin, auto-hide,
  preview, tray click, context menu dispatch, onboarding completion, and
  shortcuts.

#### 7. Wrong vs Correct

Wrong:

```tsx
if (event.payload.label !== "main") return;
```

Correct:

```tsx
if (event.payload.label !== WINDOW_LABEL.CLIPBOARD) return;
```

## Pending-Slot Pattern

When an event targets a destroyable preference window, use a Rust pending slot
if the window may not exist yet:

- Preference setting highlight: `window::open_preference_with_highlight` and
  `take_pending_preference_highlight`.
- Backup file receive: `backup::emit_received_backup` and
  `take_pending_backup`.

This avoids losing events while a WebView is being rebuilt and before React has
mounted listeners.

## macOS Window Rules

The clipboard window becomes an NSPanel in `window/macos.rs`. It can become key
without activating the app. Hidden constraints:

- The tauri-nspanel plugin must be registered before `to_panel`.
- Clipboard panel show is delayed briefly, then executed on the main thread.
- Visibility and lifecycle events for the clipboard panel are emitted after the panel
  actually shows.
- Pinned-window paste must resign key before simulated paste, then restore key
  after the target app consumes the keystroke.

Changing clipboard-window behavior on macOS requires manual testing of show/hide,
keyboard navigation, auto-hide, pinned paste, and Spaces behavior.

## Windows Window Rules

The clipboard window is non-focusable on Windows. `window/windows.rs` enables low-level
keyboard and mouse hooks while the clipboard window is visible:

- `keyboard/windows.rs` emits `keyboard://nav` for navigation keys and selected
  Ctrl shortcuts.
- `mouse/windows.rs` hides the clipboard window on outside click when auto-hide is
  allowed.
- Hiding the clipboard window disables hooks and hides context menus.

React code uses `useKeyboardEvent` so components do not need to know whether the
event came from the browser or Rust.

### Scenario: Windows Clipboard Editable Focus

#### 1. Scope / Trigger

Changing text input behavior inside the Windows clipboard window touches the
window focusability flag, low-level keyboard hooks, foreground-window restore,
Tauri command registration, and the React keyboard abstraction.

#### 2. Signatures

- Rust command:
  - `set_clipboard_window_editing(app: AppHandle, editing: bool) -> Result<()>`
- Rust platform owner:
  - `window::set_clipboard_window_editing(app_handle: &AppHandle, editing: bool) -> Result<()>`
  - `window/windows.rs::set_clipboard_window_editing(...)`
- TypeScript wrapper:
  - `setClipboardWindowEditing(editing: boolean): Promise<void>`
- Frontend owner:
  - `useClipboardWindowEditableFocus()` mounted by the clipboard page.

#### 3. Contracts

- The clipboard window remains non-focusable by default on Windows and
  `show_window` must force it back to `focusable=false` before showing.
- `editing=true` stores the current foreground HWND when it is not the clipboard
  window, disables Windows navigation-key hooks, sets the clipboard window
  focusable, and focuses it so editable controls can receive normal text input.
- `editing=false` sets the clipboard window non-focusable again, restores the
  navigation and outside-click hooks while the window remains visible, and only
  restores the stored foreground HWND when the clipboard window is still the
  foreground window.
- The frontend reports editable focus at the clipboard page level using
  `focusin`, `focusout`, `pointerdown`, window `blur`, and `visibilitychange`;
  individual input components should not call the command directly.
- `useKeyboardEvent` must let `input`, `textarea`, and `contenteditable`
  targets keep native browser keyboard behavior while editing is active.
- In the Windows clipboard window only, editable controls with
  `data-allow-global-keyboard="true"` are handoff inputs: when they receive
  navigation/function keys such as Ctrl, Arrow keys, Enter, Escape, or Tab,
  `useKeyboardEvent` must blur the control before running the global keyboard
  handler so clipboard shortcuts keep working. macOS keeps its existing browser
  keyboard bubbling path.
- Shortcut hint components must not clear their active state on the Windows
  clipboard window blur caused by Ctrl handoff while the modifier key is still
  pressed; they should remain active until the matching modifier keyup arrives.
- Programmatic focus paths such as Ctrl+F must call
  `prepareClipboardWindowEditableFocus()` before focusing the input, so Windows
  can temporarily make the clipboard window focusable first.

#### 4. Validation & Error Matrix

- Missing clipboard window -> command returns `window not found: clipboard`.
- `editing=true` with no foreground HWND or the clipboard already foreground ->
  no foreground HWND is stored.
- Stored HWND no longer exists -> skip restore and clear the stored handle.
- Windows rejects `SetForegroundWindow` -> log debug only; editing mode still
  ends and hooks are restored.
- Non-Windows platform -> command is a no-op success.

#### 5. Good/Base/Bad Cases

- Good: user opens the clipboard window from another app, focuses search or a
  note input, types normally, then blur restores the clipboard window to
  non-focusable and returns focus to the prior app.
- Good: user focuses search, presses ArrowDown, and the search input blurs
  before the list selection shortcut runs.
- Good: user focuses search, holds Ctrl, the search input blurs, and shortcut
  hints stay visible until Ctrl is released.
- Good: user presses Ctrl+F while the Windows clipboard window is non-focusable,
  the window enters editing mode first, and the search input receives focus.
- Base: no editable control is focused; Windows navigation continues through
  `keyboard://nav` and the clipboard window does not steal focus.
- Bad: leaving `WH_KEYBOARD_LL` navigation enabled during editing consumes arrow,
  Tab, Enter, or selected Ctrl shortcuts before the input can handle them.
- Bad: applying `data-allow-global-keyboard` to note or group-name editors makes
  editing keys leave the field unexpectedly.

#### 6. Tests Required

- Backend: `cargo clippy -- -D warnings` covers the Windows command and cfg
  branches; `cargo test` must still pass.
- Frontend: `pnpm tsc` and `pnpm lint` cover command mirrors and hook typing.
- Manual Windows checks: open without focus steal, type in search, type/edit a
  note, type/edit a group name, blur each field, verify the prior app receives
  focus, and verify list navigation still works after blur.

#### 7. Wrong vs Correct

Wrong:

```tsx
<Input onFocus={() => setClipboardWindowEditing(true)} />
```

Correct:

```tsx
const Clipboard = () => {
  useClipboardWindowEditableFocus();

  return <ClipboardWindowContent />;
};
```

## Platform-Specific Tauri Capabilities

Platform-specific plugin permissions must live in their own capability file with
an explicit `platforms` guard. Keep `src-tauri/capabilities/default.json`
limited to permissions valid on both macOS and Windows; otherwise Windows
capability validation can reject macOS-only identifiers before runtime cfgs take
effect.

Wrong:

```json
{
  "identifier": "default",
  "permissions": ["core:default", "macos-permissions:default"],
  "windows": ["*"]
}
```

Correct:

```json
{
  "identifier": "macos-permissions",
  "permissions": ["macos-permissions:default"],
  "platforms": ["macOS"],
  "windows": ["*"]
}
```

## Native Integration Side Effects

Settings changes that affect OS state must be applied in Rust or through the
existing command layer:

- Global shortcuts: `shortcut::apply`, `suspend`, and `resume`.
- Tray menu and language: `tray::apply`.
- Autostart: `autostart::set_enabled`.
- Dock/taskbar icon: `window::show_taskbar_icon`.
- Windows administrator launch: `admin::sync_scheduled_task` and
  `admin::launch_elevated_current_process`.

Failures in side-effect refresh after a successful settings write are logged and
do not roll back the persisted settings unless the called command itself is the
direct user action, such as `set_autostart`.

## Scenario: Windows Autostart Entry Deduplication and Duplicate Launches

### 1. Scope / Trigger

Changing Windows autostart behavior touches registry Run keys, Task Manager
startup approval records, persisted settings, and the administrator relaunch
task. Keep this Rust-owned; React only toggles `general.autoStart`.

### 2. Signatures

- Setting: `Settings.general.auto_start: bool`.
- Commands:
  - `get_autostart() -> bool`
  - `set_autostart(enabled: bool) -> ()`
- Startup sync:
  - `autostart::sync_enabled(app: &AppHandle, enabled: bool) -> Result<()>`
- Launch-source matcher:
  - `autostart::is_autostart_launch(args: &[String]) -> bool`
- Windows Run value name: app package name, currently `EcoPaste`.
- Windows autostart launch arg: `--auto-launch`.

### 3. Contracts

- Normal Windows autostart uses one effective Run entry only.
- New Run entries are written with `WindowsEnableMode::CurrentUser`, so an
  elevated EcoPaste process must not create a fresh HKLM Run value.
- Startup initialization syncs the OS Run entries with
  `settings.general.auto_start`, so upgrades can repair stale entries from older
  versions.
- Registry cleanup cannot prevent two legacy Run entries from racing before the
  primary instance finishes setup. The single-instance callback must ignore a
  secondary process whose arguments contain `--auto-launch`; it must not show
  the preference window for that process.
- A regular second launch without `--auto-launch` remains an explicit user
  action and opens the default foreground window.
- When enabling autostart and an HKLM Run value already exists, Rust first tries
  to delete HKLM. If access is denied, Rust deletes HKCU and keeps HKLM as the
  only effective startup entry instead of creating a duplicate.
- When disabling autostart, Rust must remove HKCU and HKLM Run entries. If HKLM
  cannot be removed because the process is not elevated, the direct
  `set_autostart(false)` action fails instead of persisting a false setting while
  Windows can still start the app.
- Remove matching `StartupApproved\Run` shadow values best-effort when deleting
  Run values so Task Manager and Autoruns do not keep stale EcoPaste rows.
- `admin::sync_scheduled_task` owns the `EcoPasteAdmin` and
  `EcoPasteAdminAutostart` tasks for administrator relaunch support. Both remain
  on-demand helpers, not additional logon triggers; the Run entry owns autostart.

### 4. Validation & Error Matrix

- HKCU Run missing -> cleanup succeeds.
- HKLM Run missing -> cleanup succeeds.
- Enable with removable HKLM Run -> delete HKLM, then create/update HKCU.
- Enable with inaccessible HKLM Run -> delete HKCU, return success with HKLM as
  the only startup entry.
- Disable with inaccessible HKLM Run -> command error from registry access;
  frontend should not persist `general.autoStart=false`.
- Startup sync failure after settings load -> log warning and continue app
  startup.
- Secondary instance with `--auto-launch` -> return from the single-instance
  callback without showing a window.
- Secondary instance without `--auto-launch` -> show the default foreground
  window.

### 5. Good/Base/Bad Cases

- Good: user enables autostart from an elevated process and only HKCU Run is
  created.
- Good: user upgrades with both HKCU and HKLM Run values; next startup removes
  the duplicate when permissions allow, while the already-racing secondary
  instance exits without opening preferences.
- Base: user has a locked HKLM Run value from an older install; EcoPaste keeps
  that single entry and avoids adding HKCU.
- Base: user manually launches EcoPaste while it is running; the primary
  instance opens preferences as expected.
- Bad: `auto-launch` stays in dynamic Windows mode and creates HKLM when the app
  happens to be elevated.
- Bad: disabling autostart reports success while an inaccessible HKLM Run value
  remains active.

### 6. Tests Required

- Backend: `cargo check`, `cargo clippy -- -D warnings`, and `cargo test`.
- Unit tests: `is_autostart_launch` returns true when `--auto-launch` is present
  and false for a regular executable-only argument list.
- Windows validation: enable autostart as normal user and elevated user, inspect
  Autoruns for a single EcoPaste Run entry, then disable and verify no Run entry
  remains.
- Upgrade validation: seed both HKCU and HKLM `...\Run\EcoPaste` values, launch
  EcoPaste once, and verify duplicate cleanup or single-entry fallback.
- Administrator validation: with `general.runAsAdmin=true`, confirm
  `EcoPasteAdmin` remains a relaunch helper and does not combine with duplicate
  Run entries to start multiple processes at login.

### 7. Wrong vs Correct

Wrong:

```rust
AutoLaunchBuilder::new()
    .set_app_name(&app_name)
    .set_app_path(&exe_path)
    .set_args(&[AUTO_LAUNCH_ARG])
    .build()?;
```

Correct:

```rust
let mut builder = AutoLaunchBuilder::new();
builder
    .set_app_name(&app_name)
    .set_app_path(&exe_path)
    .set_args(&[AUTO_LAUNCH_ARG]);
builder.set_windows_enable_mode(WindowsEnableMode::CurrentUser);
```

Wrong:

```rust
if backup::backup_path_from_args(&argv).is_none() {
    show_default_foreground_window(app_handle)?;
}
```

Correct:

```rust
if autostart::is_autostart_launch(&argv) {
    return;
}
show_default_foreground_window(app_handle)?;
```

## Scenario: Windows Administrator Launch

### 1. Scope / Trigger

Adding or changing administrator launch behavior touches startup order, Windows
process APIs, scheduled tasks, persisted settings, Tauri commands, TypeScript
command wrappers, and onboarding UI. Treat it as a Rust-owned platform contract,
not a frontend-only permission prompt.

### 2. Signatures

- Setting:
  - Rust `Settings.general.run_as_admin: bool`
  - TypeScript `Settings.general.runAsAdmin: boolean`
- Commands:
  - `get_run_as_admin_status() -> AdminLaunchStatus`
  - `set_run_as_admin(enabled: bool) -> Settings`
  - `restart_as_admin() -> ()`
- Response:
  - `AdminLaunchStatus.configured: bool`
  - `AdminLaunchStatus.runningAsAdmin: bool`
  - `AdminLaunchStatus.taskReady: bool`
- Internal restart marker:
  - `--ecopaste-admin-restarted`

### 3. Contracts

- `runAsAdmin` records persistent user intent. The current Windows access token
  is the source of truth for whether EcoPaste is actually elevated.
- Windows elevation detection uses `OpenProcessToken` and
  `GetTokenInformation(TokenElevation)`.
- Release startup calls the admin auto-elevation check before normal Tauri setup
  initializes windows, tray, database, clipboard watcher, or hooks.
- Enabling administrator launch while elevated and release startup with
  `runAsAdmin=true` both sync the two highest-privilege scheduled tasks. This
  refreshes the legacy manual task and registers the autostart helper on upgrade.
- Fixed actions never depend on the creating process's arguments:
  `EcoPasteAdmin` uses `--ecopaste-admin-restarted`; `EcoPasteAdminAutostart`
  uses `--ecopaste-admin-restarted --auto-launch`.
- `taskReady` requires both tasks to exist and point at the current executable.
- If `runAsAdmin=true` and the process is not elevated, Rust tries to launch an
  elevated process and exits the unelevated process only after launch succeeds.
- Relaunch allows fixed tasks only for no arguments, the internal restart
  marker, `--auto-launch`, or their combination. Presence of `--auto-launch`
  selects `EcoPasteAdminAutostart`; otherwise it selects `EcoPasteAdmin`.
  Query, path validation, and run must use that same selected task. Any file-open
  path or other argument falls back to `ShellExecuteW` with `runas`, preserving
  the original arguments. Both fixed actions retain the restart marker to
  prevent repeated startup elevation attempts.
- React renders `AdminLaunchStatus` and sends user intent through command
  wrappers. It must not call Windows APIs or infer token elevation itself.

### 4. Validation & Error Matrix

- `restart_as_admin` on non-Windows -> command error `administrator launch is only available on Windows`.
- UAC cancelled or elevated launch fails -> command error
  `administrator permission request was cancelled or failed`; current process
  remains open.
- Scheduled task path mismatch -> task is not considered ready; fallback to UAC.
- `set_run_as_admin(false)` while elevated -> Rust attempts deletion of both
  scheduled tasks best-effort after settings update, even if one attempt fails.
- Early settings or storage manifest read failure -> auto-elevation is skipped
  and normal startup continues.

### 5. Good/Base/Bad Cases

- Good: onboarding on Windows shows the administrator card as actionable, the
  user grants access, Rust persists `runAsAdmin`, launches an elevated instance,
  exits the old process, and the elevated instance reopens onboarding with the
  card granted.
- Base: app already runs elevated; onboarding reports granted and startup keeps
  the scheduled task aligned with the current executable.
- Bad: frontend marks administrator permission granted because `runAsAdmin=true`
  even though `runningAsAdmin=false`.

### 6. Tests Required

- Backend: unit-test Windows command-line quoting for scheduled task / UAC
  arguments, both fixed task actions, and real routing for no args, autostart,
  either marker order, file associations, and unsupported arguments. Tests must
  not run scheduled tasks, request elevation, or modify system task state.
- Backend: `cargo clippy -- -D warnings` must pass on Windows-specific code.
- Backend: `cargo test` must include settings default coverage for
  `general.run_as_admin`.
- Frontend: `pnpm tsc` must cover `AdminLaunchStatus` and the mirrored
  `runAsAdmin` setting.
- Manual Windows validation is required for UAC approval/cancel and scheduled
  task launch behavior.

### 7. Wrong vs Correct

Wrong:

```tsx
const granted = settings.general.runAsAdmin;
```

Correct:

```tsx
const status = await getRunAsAdminStatus();
const granted = status.runningAsAdmin;
```

## Scenario: Auto Updater Window

### 1. Scope / Trigger

Adding or changing automatic update behavior touches Tauri plugin setup, Rust
commands, stored update settings, Tauri window lifecycle, TypeScript command
mirrors, and localized UI strings. Treat it as a cross-layer contract change,
not a frontend-only preference action.

### 2. Signatures

- Window label:
  - Rust `window::UPDATE_WINDOW_LABEL = "update"`
  - TypeScript `WINDOW_LABEL.UPDATE = "update"`
- Commands:
  - `get_update_status() -> AppUpdateStatus`
  - `check_for_updates() -> AppUpdateStatus`
  - `download_update(version: String) -> UpdateMetadata`
  - `install_update(version: String) -> ()`
  - `skip_update_version(version: String) -> AppUpdateStatus`
  - `open_update_window() -> ()`
- Event:
  - `update://progress` with `{ downloaded, total, progress }`
- Settings fields:
  - `update.autoCheck: bool`
  - `update.includeBeta: bool`
  - `update.includeNightly: bool`
  - `update.frequency: daily | weekly | monthly`
  - `update.lastCheckedAt: Option<String>`
  - `update.skippedVersion: Option<String>`

### 3. Contracts

- Rust owns update checks, signature verification, downloaded bytes, install,
  auto-check throttling, skipped-version persistence, and automatic update
  window display.
- On macOS, `tauri-plugin-updater` installation APIs replace the application
  files and return without relaunching the process. `install_update` must
  request an application restart after installation succeeds and must not
  restart after an install error. On Windows the updater exits and delegates
  relaunch to the spawned installer before this call can return.
- `update::schedule_auto_check` must run for the whole app session: it performs
  the initial delayed background check, then computes the next due time from
  `update.lastCheckedAt + update.frequency` so the configured cadence is honored
  even when EcoPaste stays open for days. The scheduler may cap long sleeps to
  refresh settings, but network checks must still pass through Rust-side
  `should_auto_check` throttling.
- `update.skippedVersion` suppresses that exact version for both manual and
  automatic checks; a newer different version is shown normally.
- React renders `AppUpdateStatus` and sends user intent through command wrappers
  only. Do not call updater plugin APIs directly from React components.
- Stable endpoint default:
  `https://releases.ecopaste.cn/update?channel=stable`.
- Optional environment overrides:
  - `ECOPASTE_UPDATE_ENDPOINT`: stable channel endpoint.
  - `ECOPASTE_UPDATE_BETA_ENDPOINT`: beta channel endpoint; defaults to
    `https://releases.ecopaste.cn/update?channel=beta` and is checked before
    the stable endpoint when `includeBeta` is true.
  - `ECOPASTE_UPDATE_NIGHTLY_ENDPOINT`: nightly channel endpoint; defaults to
    `https://releases.ecopaste.cn/update?channel=nightly` and is checked before
    beta and stable endpoints when `includeNightly` is true.
  - `TAURI_UPDATER_PUBLIC_KEY` or `TAURI_SIGNING_PUBLIC_KEY`: public updater
    verification key injected into the plugin at runtime.
  - `TAURI_SIGNING_PRIVATE_KEY`: build-time private key used by Tauri bundling
    to sign artifacts; never read it from app runtime code.

### 4. Validation & Error Matrix

- Invalid endpoint URL -> command error `update endpoint is invalid: ...`.
- No configured endpoint or updater build failure -> command error from Rust
  updater setup.
- No pending update when downloading/installing -> `no update is ready`.
- Version mismatch between UI request and Rust pending update ->
  `the selected update is no longer current`.
- Install before successful download -> `update is not downloaded`.
- Install failure -> propagated command error; the current process stays open.
- Successful macOS install -> Rust logs the installed version and requests an
  application restart. Successful Windows install -> the updater exits and the
  installer owns relaunch.
- Signature mismatch -> propagated from `tauri-plugin-updater` download
  verification; frontend should show the command error and remain in error
  state.

### 5. Good/Base/Bad Cases

- Good: user opens About -> Check for Updates; Rust opens the `update` window,
  checks stable/beta endpoint according to settings, downloads through Rust,
  emits progress, verifies signature, installs, then requests an application
  restart.
- Base: no update is available; Rust clears pending update state and the window
  renders the latest-version state.
- Bad: React stores downloaded bytes, performs signature checks, or decides
  auto-check frequency locally.
- Bad: Rust assumes `Update::install` or `Update::download_and_install`
  automatically relaunches the application on macOS.

### 6. Tests Required

- Backend checks: `cargo clippy -- -D warnings` and `cargo test`.
- Auto-check scheduling changes must verify that the background task loops after
  the first check and still delegates frequency/auto-check gating to
  `should_auto_check`.
- Frontend checks: `pnpm tsc` and `pnpm lint`.
- Packaging/build check: `pnpm build` for route and locale integration.
- Manual signed-artifact check must assert that a successful install terminates
  the old process, relaunches EcoPaste, and reports the new version after
  startup.
- Manual update-server checks when release artifacts exist:
  - mock endpoint returns newer signed version -> update found and install path
    starts.
  - mock endpoint returns bad signature -> download refuses install.
  - `includeBeta=false` -> stable endpoint only.
  - `includeBeta=true` -> beta endpoint is attempted before stable fallback.
  - `includeNightly=true` -> nightly endpoint is attempted before beta and
    stable fallback.

### 7. Wrong vs Correct

Wrong:

```tsx
import { check } from "@tauri-apps/plugin-updater";

await check();
```

Correct:

```tsx
import { checkForUpdates } from "@/commands";

const status = await checkForUpdates();
```

Wrong:

```rust
app.state::<UpdateState>().install_downloaded(&version)?;
Ok(())
```

Correct:

```rust
app.state::<UpdateState>().install_downloaded(&version)?;
app.request_restart();
Ok(())
```
