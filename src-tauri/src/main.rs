#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app_logic;

use app_logic::{
    apply_pending_bind, default_notice, handle_key_release, key_to_label, set_notice,
    target_to_ui_label, to_ui_state, HotKey, KeyBindingTarget, SharedState, UiState,
};
use core::mem::size_of;
use std::env;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use tauri::Manager;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    KBDLLHOOKSTRUCT, MSG, SW_SHOW, WH_KEYBOARD_LL, WM_KEYUP, WM_SYSKEYUP,
};

#[derive(Clone)]
struct AppShared {
    inner: Arc<Mutex<SharedState>>,
    click_notify: Arc<Condvar>,
}

impl AppShared {
    fn new(is_elevated: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedState {
                cps: 13,
                running: false,
                inv_paused: false,
                inventory_key: HotKey::KEY_E,
                toggle_key: HotKey::F4,
                pending_bind: None,
                notice: default_notice(is_elevated),
                is_elevated,
            })),
            click_notify: Arc::new(Condvar::new()),
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

// Sender global utilisé par le callback WH_KEYBOARD_LL.
static GLOBAL_KEY_TX: OnceLock<Mutex<Option<mpsc::Sender<HotKey>>>> = OnceLock::new();

unsafe extern "system" fn keyboard_proc_global(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let w = wparam.0 as u32;
        if w == WM_KEYUP || w == WM_SYSKEYUP {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = HotKey(kb.vkCode);
            if let Some(store) = GLOBAL_KEY_TX.get() {
                if let Ok(guard) = store.lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.send(vk);
                    }
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Installe un WH_KEYBOARD_LL pur — aucun hook souris.
/// Le callback poste les VK codes dans `key_tx` et retourne immédiatement.
fn install_keyboard_hook(key_tx: mpsc::Sender<HotKey>) {
    thread::spawn(move || {
        // Le hook vit dans un thread avec message loop Windows.
        let store = GLOBAL_KEY_TX.get_or_init(|| Mutex::new(None));
        if let Ok(mut guard) = store.lock() {
            *guard = Some(key_tx);
        }

        let _hook = unsafe {
            let hmod = GetModuleHandleW(None).unwrap_or_default();
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc_global),
                Some(HINSTANCE(hmod.0)),
                0,
            )
            .expect("Failed to install keyboard hook")
        };

        // Message loop nécessaire pour que le hook reçoive les événements.
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
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
    if running {
        shared.click_notify.notify_one();
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

#[tauri::command]
fn set_key_bind(
    target: String,
    key_code: u32,
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppShared>,
) -> Result<UiState, String> {
    if !(1..=255).contains(&key_code) {
        return Err("Invalid key code".to_string());
    }

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
        let _ = apply_pending_bind(&mut guard, HotKey(key_code));
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
            begin_key_bind,
            set_key_bind
        ])
        .setup(|app| {
            let app_handle = app.handle();
            let state_for_hotkeys = app.state::<AppShared>().inner.clone();

            // Thread autoclick — dormant via Condvar jusqu'à running == true.
            {
                let click_state = app.state::<AppShared>().inner.clone();
                let click_notify = app.state::<AppShared>().click_notify.clone();
                thread::spawn(move || loop {
                    let cps = {
                        let guard = click_notify
                            .wait_while(click_state.lock().unwrap(), |s| !s.running)
                            .unwrap();
                        guard.cps
                    };
                    let still_running =
                        click_state.lock().map(|g| g.running).unwrap_or(false);
                    if still_running {
                        click_once(cps);
                    }
                });
            }

            // Channel clavier : hook → sender (immédiat) → thread consommateur.
            let (key_tx, key_rx) = mpsc::channel::<HotKey>();

            // Thread consommateur des touches.
            {
                let process_state = state_for_hotkeys.clone();
                let process_handle = app_handle.clone();
                let process_notify = app.state::<AppShared>().click_notify.clone();
                thread::spawn(move || {
                    for key in key_rx {
                        let mut should_emit = false;
                        let mut started_running = false;
                        if let Ok(mut guard) = process_state.lock() {
                            should_emit = handle_key_release(&mut guard, key);
                            started_running = guard.running;
                        }
                        if should_emit {
                            if started_running {
                                process_notify.notify_one();
                            }
                            emit_state(&process_handle, &process_state);
                        }
                    }
                });
            }

            // Hook WH_KEYBOARD_LL pur — zéro hook souris.
            install_keyboard_hook(key_tx);

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
            inventory_key: HotKey::KEY_E,
            toggle_key: HotKey::F4,
            pending_bind: None,
            notice: String::new(),
            is_elevated: true,
        }
    }

    #[test]
    fn escape_cancels_pending_bind() {
        let mut state = sample_state();
        state.pending_bind = Some(KeyBindingTarget::Toggle);
        assert!(apply_pending_bind(&mut state, HotKey::ESCAPE));
        assert_eq!(state.pending_bind, None);
        assert_eq!(state.toggle_key, HotKey::F4);
    }

    #[test]
    fn duplicate_binding_is_rejected() {
        let mut state = sample_state();
        state.pending_bind = Some(KeyBindingTarget::Toggle);
        assert!(apply_pending_bind(&mut state, HotKey::KEY_E));
        assert_eq!(state.pending_bind, Some(KeyBindingTarget::Toggle));
        assert_eq!(state.toggle_key, HotKey::F4);
    }

    #[test]
    fn inventory_pause_toggles_cleanly() {
        let mut state = sample_state();
        state.running = true;
        assert!(handle_key_release(&mut state, HotKey::KEY_E));
        assert!(!state.running);
        assert!(state.inv_paused);
        assert!(handle_key_release(&mut state, HotKey::KEY_E));
        assert!(state.running);
        assert!(!state.inv_paused);
    }

    #[test]
    fn key_label_e_is_readable() {
        assert_eq!(key_to_label(HotKey::KEY_E), "E");
    }

    #[test]
    fn key_label_space_is_readable() {
        assert_eq!(key_to_label(HotKey(0x20)), "Space");
    }

    #[test]
    fn key_label_f4_is_readable() {
        assert_eq!(key_to_label(HotKey::F4), "F4");
    }
}
