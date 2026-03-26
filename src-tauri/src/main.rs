#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]


use rdev::{listen, EventType, Key};
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::Manager;
use windows::core::PCWSTR;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyBindingTarget {
    Inventory,
    Toggle,
}

struct SharedState {
    cps: u32,
    running: bool,
    inv_paused: bool,
    inventory_key: Key,
    toggle_key: Key,
    pending_bind: Option<KeyBindingTarget>,
    notice: String,
    is_elevated: bool,
}

#[derive(Clone)]
struct AppShared(Arc<Mutex<SharedState>>);

impl AppShared {
    fn new(is_elevated: bool) -> Self {
        Self(Arc::new(Mutex::new(SharedState {
            cps: 13,
            running: false,
            inv_paused: false,
            inventory_key: Key::KeyE,
            toggle_key: Key::F4,
            pending_bind: None,
            notice: default_notice(is_elevated),
            is_elevated,
        })))
    }
}

#[derive(Serialize, Clone)]
struct UiState {
    cps: u32,
    running: bool,
    inv_paused: bool,
    status: String,
    inventory_key: String,
    toggle_key: String,
    pending_bind: Option<String>,
    notice: String,
    is_elevated: bool,
}

fn default_notice(is_elevated: bool) -> String {
    if is_elevated {
        "You can assign any keyboard key.".to_string()
    } else {
        "Administrator mode is required for this app. Please accept the UAC prompt on launch.".to_string()
    }
}

fn key_to_label(key: Key) -> String {
    match key {
        Key::Alt => "Alt",
        Key::AltGr => "Alt Gr",
        Key::BackQuote => "`",
        Key::BackSlash => "\\",
        Key::Backspace => "Backspace",
        Key::CapsLock => "Caps Lock",
        Key::Comma => ",",
        Key::ControlLeft => "Left Ctrl",
        Key::ControlRight => "Right Ctrl",
        Key::Delete => "Delete",
        Key::Dot => ".",
        Key::DownArrow => "Down Arrow",
        Key::End => "End",
        Key::Escape => "Escape",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Home => "Home",
        Key::Insert => "Insert",
        Key::KeyA => "A",
        Key::KeyB => "B",
        Key::KeyC => "C",
        Key::KeyD => "D",
        Key::KeyE => "E",
        Key::KeyF => "F",
        Key::KeyG => "G",
        Key::KeyH => "H",
        Key::KeyI => "I",
        Key::KeyJ => "J",
        Key::KeyK => "K",
        Key::KeyL => "L",
        Key::KeyM => "M",
        Key::KeyN => "N",
        Key::KeyO => "O",
        Key::KeyP => "P",
        Key::KeyQ => "Q",
        Key::KeyR => "R",
        Key::KeyS => "S",
        Key::KeyT => "T",
        Key::KeyU => "U",
        Key::KeyV => "V",
        Key::KeyW => "W",
        Key::KeyX => "X",
        Key::KeyY => "Y",
        Key::KeyZ => "Z",
        Key::LeftArrow => "Left Arrow",
        Key::LeftBracket => "[",
        Key::MetaLeft => "Left Win",
        Key::MetaRight => "Right Win",
        Key::Minus => "-",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::NumLock => "Num Lock",
        Key::PageDown => "Page Down",
        Key::PageUp => "Page Up",
        Key::Pause => "Pause",
        Key::PrintScreen => "Print Screen",
        Key::Quote => "'",
        Key::Return => "Enter",
        Key::RightArrow => "Right Arrow",
        Key::RightBracket => "]",
        Key::ScrollLock => "Scroll Lock",
        Key::SemiColon => ";",
        Key::ShiftLeft => "Left Shift",
        Key::ShiftRight => "Right Shift",
        Key::Slash => "/",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "Up Arrow",
        Key::Kp0 => "Numpad 0",
        Key::Kp1 => "Numpad 1",
        Key::Kp2 => "Numpad 2",
        Key::Kp3 => "Numpad 3",
        Key::Kp4 => "Numpad 4",
        Key::Kp5 => "Numpad 5",
        Key::Kp6 => "Numpad 6",
        Key::Kp7 => "Numpad 7",
        Key::Kp8 => "Numpad 8",
        Key::Kp9 => "Numpad 9",
        Key::KpDelete => "Numpad .",
        Key::KpDivide => "Numpad /",
        Key::KpMinus => "Numpad -",
        Key::KpMultiply => "Numpad *",
        Key::KpPlus => "Numpad +",
        Key::KpReturn => "Numpad Enter",
        other => return format!("{other:?}"),
    }
    .to_string()
}

fn target_to_ui_label(target: KeyBindingTarget) -> &'static str {
    match target {
        KeyBindingTarget::Inventory => "Inventory pause",
        KeyBindingTarget::Toggle => "Toggle autoclick",
    }
}

fn to_ui_state(inner: &SharedState) -> UiState {
    let status = if inner.running {
        "Active"
    } else if inner.inv_paused {
        "Paused (inventory)"
    } else {
        "Stopped"
    };

    UiState {
        cps: inner.cps,
        running: inner.running,
        inv_paused: inner.inv_paused,
        status: status.to_string(),
        inventory_key: key_to_label(inner.inventory_key),
        toggle_key: key_to_label(inner.toggle_key),
        pending_bind: inner.pending_bind.map(|target| match target {
            KeyBindingTarget::Inventory => "inventory".to_string(),
            KeyBindingTarget::Toggle => "toggle".to_string(),
        }),
        notice: inner.notice.clone(),
        is_elevated: inner.is_elevated,
    }
}

fn emit_state(app: &tauri::AppHandle, shared: &Arc<Mutex<SharedState>>) {
    if let Ok(guard) = shared.lock() {
        let _ = app.emit_all("state-updated", to_ui_state(&guard));
    }
}

fn click_once(cps: u32) {
    let safe_cps = cps.max(1);
    let delay_ms = (1000u64 / safe_cps as u64).max(1);

    // Only click flags are injected. No move / wheel event is ever sent.
    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    thread::sleep(Duration::from_millis(delay_ms));
}

fn extract_relevant_key(event_type: EventType) -> Option<Key> {
    match event_type {
        EventType::KeyRelease(key) => Some(key),
        _ => None,
    }
}

fn set_notice(inner: &mut SharedState, notice: impl Into<String>) {
    inner.notice = notice.into();
}

fn is_binding_conflict(inner: &SharedState, target: KeyBindingTarget, key: Key) -> bool {
    match target {
        KeyBindingTarget::Inventory => key == inner.toggle_key,
        KeyBindingTarget::Toggle => key == inner.inventory_key,
    }
}

fn apply_pending_bind(inner: &mut SharedState, key: Key) -> bool {
    let Some(target) = inner.pending_bind else {
        return false;
    };

    if key == Key::Escape {
        inner.pending_bind = None;
        set_notice(inner, "Key binding canceled.");
        return true;
    }

    if is_binding_conflict(inner, target, key) {
        set_notice(inner, "This key is already used by the other hotkey.");
        return true;
    }

    match target {
        KeyBindingTarget::Inventory => inner.inventory_key = key,
        KeyBindingTarget::Toggle => inner.toggle_key = key,
    }

    inner.pending_bind = None;
    set_notice(
        inner,
        format!("{} is now set to {}.", target_to_ui_label(target), key_to_label(key)),
    );
    true
}

fn handle_key_release(inner: &mut SharedState, key: Key) -> bool {
    if inner.pending_bind.is_some() {
        return apply_pending_bind(inner, key);
    }

    if key == inner.toggle_key {
        inner.running = !inner.running;
        inner.inv_paused = false;
        set_notice(
            inner,
            if inner.running {
                "Autoclick enabled."
            } else {
                "Autoclick disabled."
            },
        );
        return true;
    }

    if key == inner.inventory_key {
        if inner.running {
            inner.running = false;
            inner.inv_paused = true;
            set_notice(inner, "Inventory pause enabled.");
            return true;
        }

        if inner.inv_paused {
            inner.running = true;
            inner.inv_paused = false;
            set_notice(inner, "Resumed after inventory pause.");
            return true;
        }
    }

    false
}

fn is_user_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn relaunch_as_admin() {
    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return,
    };

    let operation = to_wide("runas");
    let file = to_wide(&exe.to_string_lossy());

    unsafe {
        let _ = ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        );
    }
}

#[tauri::command]
fn get_state(shared: tauri::State<'_, AppShared>) -> Result<UiState, String> {
    shared
        .0
        .lock()
        .map(|guard| to_ui_state(&guard))
        .map_err(|_| "State unavailable".to_string())
}

#[tauri::command]
fn set_cps(
    cps: u32,
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppShared>,
) -> Result<UiState, String> {
    {
        let mut guard = shared
            .0
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        guard.cps = cps.clamp(1, 100);
        let current_cps = guard.cps;
        set_notice(&mut guard, format!("Speed set to {} CPS.", current_cps));
    }
    emit_state(&app, &shared.0);
    get_state(shared)
}

#[tauri::command]
fn set_running(
    running: bool,
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppShared>,
) -> Result<UiState, String> {
    {
        let mut guard = shared
            .0
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        guard.running = running;
        if running {
            guard.inv_paused = false;
            set_notice(&mut guard, "Autoclick enabled from UI.");
        } else {
            set_notice(&mut guard, "Autoclick disabled from UI.");
        }
    }
    emit_state(&app, &shared.0);
    get_state(shared)
}

#[tauri::command]
fn begin_key_bind(
    target: String,
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppShared>,
) -> Result<UiState, String> {
    {
        let mut guard = shared
            .0
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        let binding_target = match target.as_str() {
            "inventory" => KeyBindingTarget::Inventory,
            "toggle" => KeyBindingTarget::Toggle,
            _ => return Err("Invalid bind target".to_string()),
        };

        guard.pending_bind = Some(binding_target);
        set_notice(
            &mut guard,
            format!(
                "Press any key to change {}. Escape cancels.",
                target_to_ui_label(binding_target)
            ),
        );
    }
    emit_state(&app, &shared.0);
    get_state(shared)
}

fn main() {
    if !is_user_elevated() {
        relaunch_as_admin();
        return;
    }

    let shared = AppShared::new(true);

    let app = tauri::Builder::default()
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_cps,
            set_running,
            begin_key_bind
        ])
        .setup(|app| {
            let app_handle = app.handle();
            let state_for_click = app.state::<AppShared>().0.clone();
            let state_for_hotkeys = app.state::<AppShared>().0.clone();

            {
                thread::spawn(move || loop {
                    let (running, cps) = {
                        if let Ok(guard) = state_for_click.lock() {
                            (guard.running, guard.cps)
                        } else {
                            (false, 10)
                        }
                    };

                    if running {
                        click_once(cps);
                    } else {
                        thread::sleep(Duration::from_millis(10));
                    }
                });
            }

            {
                let app_handle_for_hotkeys = app_handle.clone();
                thread::spawn(move || {
                    let listener_state = state_for_hotkeys.clone();
                    let listener_handle = app_handle_for_hotkeys.clone();
                    let listen_result = listen(move |event| {
                        let Some(key) = extract_relevant_key(event.event_type) else {
                            return;
                        };

                        let mut should_emit = false;

                        if let Ok(mut guard) = listener_state.lock() {
                            should_emit = handle_key_release(&mut guard, key);
                        }

                        if should_emit {
                            emit_state(&listener_handle, &listener_state);
                        }
                    });

                    if let Err(error) = listen_result {
                        if let Ok(mut guard) = state_for_hotkeys.lock() {
                            set_notice(
                                &mut guard,
                                format!("Global keyboard listener failed: {error:?}"),
                            );
                        }
                        emit_state(&app_handle_for_hotkeys, &state_for_hotkeys);
                    }
                });
            }

            emit_state(&app_handle, &app.state::<AppShared>().0);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Erreur au build de l'application Tauri");

    app.run(|_, _| {});
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> SharedState {
        SharedState {
            cps: 13,
            running: false,
            inv_paused: false,
            inventory_key: Key::KeyE,
            toggle_key: Key::F4,
            pending_bind: None,
            notice: String::new(),
            is_elevated: true,
        }
    }

    #[test]
    fn escape_cancels_pending_bind() {
        let mut state = sample_state();
        state.pending_bind = Some(KeyBindingTarget::Toggle);

        assert!(apply_pending_bind(&mut state, Key::Escape));
        assert_eq!(state.pending_bind, None);
        assert_eq!(state.toggle_key, Key::F4);
    }

    #[test]
    fn duplicate_binding_is_rejected() {
        let mut state = sample_state();
        state.pending_bind = Some(KeyBindingTarget::Toggle);

        assert!(apply_pending_bind(&mut state, Key::KeyE));
        assert_eq!(state.pending_bind, Some(KeyBindingTarget::Toggle));
        assert_eq!(state.toggle_key, Key::F4);
    }

    #[test]
    fn inventory_pause_toggles_cleanly() {
        let mut state = sample_state();
        state.running = true;

        assert!(handle_key_release(&mut state, Key::KeyE));
        assert!(!state.running);
        assert!(state.inv_paused);

        assert!(handle_key_release(&mut state, Key::KeyE));
        assert!(state.running);
        assert!(!state.inv_paused);
    }

    #[test]
    fn mouse_move_event_is_ignored() {
        assert_eq!(
            extract_relevant_key(EventType::MouseMove { x: 42.0, y: 24.0 }),
            None
        );
    }

    #[test]
    fn only_key_release_events_are_used() {
        assert_eq!(extract_relevant_key(EventType::KeyPress(Key::F4)), None);
        assert_eq!(extract_relevant_key(EventType::KeyRelease(Key::F4)), Some(Key::F4));
    }

    #[test]
    fn key_labels_are_human_readable() {
        assert_eq!(key_to_label(Key::KeyE), "E");
        assert_eq!(key_to_label(Key::Space), "Space");
        assert_eq!(key_to_label(Key::F4), "F4");
    }
}
