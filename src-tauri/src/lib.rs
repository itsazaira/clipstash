//! ClipStash, by Its Azaira. Your last 50 copies, one shortcut away.
//!
//! No network calls of any kind: history lives in `history.json` in the app data folder.

mod paste;
mod store;
mod watcher;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use store::{Snapshot, Store};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

const POPUP: &str = "popup";

/// True when the popup was opened with the shortcut, so a pick pastes into the app the user was in.
/// Opened from the tray, taskbar or a second launch, there is no such app and a pick only copies.
static PASTE_TARGET: AtomicBool = AtomicBool::new(false);
/// The popup is centred the first time only, so it stays wherever the user drags it.
static POSITIONED: AtomicBool = AtomicBool::new(false);

/// The only URLs the app will ever open, all from the About page.
const LINKS: &[&str] = &[
    "https://itsazaira.com",
    "https://github.com/itsazaira/clipstash",
    "https://www.instagram.com/itsazaira/",
    "https://www.linkedin.com/company/itsazaira",
    "mailto:hello@itsazaira.com",
];

pub struct AppState {
    pub store: Mutex<Store>,
    pause_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

/// Tells every window the history changed.
pub fn emit_state(app: &AppHandle) {
    let snapshot = app.state::<AppState>().store.lock().unwrap().snapshot();
    let _ = app.emit("state-changed", snapshot);
}

fn show_popup(app: &AppHandle, from_shortcut: bool) {
    let Some(window) = app.get_webview_window(POPUP) else { return };
    if from_shortcut {
        paste::remember_foreground();
    }
    PASTE_TARGET.store(from_shortcut, Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    let _ = app.show();
    if !POSITIONED.swap(true, Ordering::SeqCst) {
        let _ = window.center();
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("popup-shown", ());
}

fn hide_popup_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPUP) {
        let _ = window.hide();
    }
}

/// The shortcut closes the popup when it is open and in front; otherwise it brings it forward.
fn toggle_popup(app: &AppHandle, from_shortcut: bool) {
    let in_front = app
        .get_webview_window(POPUP)
        .map(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
        .unwrap_or(false);
    if in_front {
        hide_popup_window(app);
    } else {
        show_popup(app, from_shortcut);
    }
}

fn apply_paused(app: &AppHandle, paused: bool) {
    let state = app.state::<AppState>();
    state.store.lock().unwrap().set_paused(paused);
    if let Some(item) = state.pause_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(paused);
    }
    emit_state(app);
}

fn popup_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let modifiers = Modifiers::SUPER | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let modifiers = Modifiers::CONTROL | Modifiers::SHIFT;
    Shortcut::new(Some(modifiers), Code::KeyV)
}

fn build_tray(app: &AppHandle, paused: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open ClipStash", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause recording", true, paused, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit ClipStash", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pause, &PredefinedMenuItem::separator(app)?, &quit])?;
    *app.state::<AppState>().pause_item.lock().unwrap() = Some(pause);

    // Mac menu bar icons are monochrome templates; Windows uses the full colour app icon.
    #[cfg(target_os = "macos")]
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray/44x44.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = app.default_window_icon().cloned().expect("app icon is bundled");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("ClipStash")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_popup(app, false),
            "pause" => {
                let paused = !app.state::<AppState>().store.lock().unwrap().data.paused;
                apply_paused(app, paused);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_popup(tray.app_handle(), false);
            }
        })
        .build(app)?;
    Ok(())
}

#[tauri::command]
fn get_state(state: State<AppState>) -> Snapshot {
    state.store.lock().unwrap().snapshot()
}

/// Copies the item back to the clipboard. When the popup was opened with the shortcut it also hides
/// the popup and pastes into the previous app ("pasted"); otherwise it only copies ("copied").
/// Returns "needs-permission" on macOS when Accessibility access is missing (the item is still copied).
#[tauri::command]
fn paste_item(app: AppHandle, state: State<AppState>, id: u64) -> Result<&'static str, String> {
    let text = {
        let mut store = state.store.lock().unwrap();
        let text = store.touch(id).ok_or("Item not found")?;
        // So the watcher does not save this copy again.
        store.last_seen = text.clone();
        text
    };
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .map_err(|e| e.to_string())?;
    emit_state(&app);

    if !PASTE_TARGET.load(Ordering::SeqCst) {
        return Ok("copied");
    }
    if !paste::has_permission() {
        return Ok("needs-permission");
    }
    hide_popup_window(&app);
    #[cfg(target_os = "macos")]
    let _ = app.hide();
    std::thread::spawn(paste::paste_into_previous);
    Ok("pasted")
}

#[tauri::command]
fn toggle_pin(app: AppHandle, state: State<AppState>, id: u64) {
    state.store.lock().unwrap().toggle_pin(id);
    emit_state(&app);
}

#[tauri::command]
fn set_paused(app: AppHandle, paused: bool) {
    apply_paused(&app, paused);
}

#[tauri::command]
fn clear_all(app: AppHandle, state: State<AppState>) {
    state.store.lock().unwrap().clear();
    emit_state(&app);
}

#[tauri::command]
fn set_max_items(app: AppHandle, state: State<AppState>, max: usize) -> Result<(), String> {
    if !state.store.lock().unwrap().set_max_items(max) {
        return Err("Unsupported history size".into());
    }
    emit_state(&app);
    Ok(())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    hide_popup_window(&app);
}

#[tauri::command]
fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    if !LINKS.contains(&url.as_str()) {
        return Err("Link not allowed".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn has_accessibility() -> bool {
    paste::has_permission()
}

#[tauri::command]
fn request_accessibility() {
    paste::request_permission();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Launching ClipStash again just opens the popup of the running copy.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_popup(app, false)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_popup(app, true);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Menu bar only, no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let store = Store::load(app.path().app_data_dir()?.join("history.json"));
            let first_run = !store.data.first_run_done;
            let paused = store.data.paused;
            app.manage(AppState { store: Mutex::new(store), pause_item: Mutex::new(None) });

            // Start at login is on by default (skipped for dev builds).
            if first_run {
                if !cfg!(debug_assertions) {
                    let _ = app.autolaunch().enable();
                }
                let state = app.state::<AppState>();
                let mut store = state.store.lock().unwrap();
                store.data.first_run_done = true;
                store.save();
            }

            build_tray(app.handle(), paused)?;
            if let Err(e) = app.global_shortcut().register(popup_shortcut()) {
                eprintln!("ClipStash: could not register the shortcut: {e}");
            }
            watcher::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != POPUP {
                return;
            }
            // The popup stays open until the user closes it; closing only hides it to the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            paste_item,
            toggle_pin,
            set_paused,
            clear_all,
            get_autostart,
            set_autostart,
            hide_popup,
            set_max_items,
            open_link,
            has_accessibility,
            request_accessibility,
        ])
        .build(tauri::generate_context!())
        .expect("error while building ClipStash");

    app.run(|_app, event| {
        // Closing windows must not quit a tray app. Only Quit does.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
