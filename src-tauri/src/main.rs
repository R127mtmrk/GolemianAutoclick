#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app_logic;

use app_logic::{
    apply_pending_bind, default_notice, extract_relevant_key, handle_key_release, key_to_label,
    set_notice, target_to_ui_label, to_ui_state, KeyBindingTarget, SharedState, UiState,
};
use rdev::{listen, EventType, Key};
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

#[derive(Clone)]
struct AppShared {
    inner: Arc<Mutex<SharedState>>,
}

impl AppShared {
    fn new(is_elevated: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedState {
                cps: 13,
                running: false,
                inv_paused: false,
                inventory_key: Key::KeyE,
                toggle_key: Key::F4,
                pending_bind: None,
                notice: default_notice(is_elevated),
                is_elevated,
            })),
        }
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
        let _ = SendInput(&inputs, size_of::<INPUT>() as i32);
    }

    thread::sleep(Duration::from_millis(delay_ms));
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
        .inner
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
            .inner
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        guard.cps = cps.clamp(1, 100);
        let current_cps = guard.cps;
        set_notice(&mut guard, format!("Speed set to {} CPS.", current_cps));
    }
    emit_state(&app, &shared.inner);
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
            .inner
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
    emit_state(&app, &shared.inner);
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
            .inner
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
    emit_state(&app, &shared.inner);
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
            let state_for_click = app.state::<AppShared>().inner.clone();
            let state_for_hotkeys = app.state::<AppShared>().inner.clone();

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

            emit_state(&app_handle, &app.state::<AppShared>().inner);
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
