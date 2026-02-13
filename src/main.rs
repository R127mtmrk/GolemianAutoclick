use eframe::egui;
use rdev::{listen, simulate, Button, EventType, Key};
use std::env;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

struct ClickSettings {
    cps: u32,
}

struct AutoclickerApp {
    cps: f32,
    running: bool,
    status: String,
    running_flag: Arc<AtomicBool>,
    settings: Arc<Mutex<ClickSettings>>,
    inventory: String,
    hotkey: Arc<Mutex<Key>>,
    inv_paused: Arc<AtomicBool>,
}

impl AutoclickerApp {
    fn new(
        running_flag: Arc<AtomicBool>,
        settings: Arc<Mutex<ClickSettings>>,
        hotkey: Arc<Mutex<Key>>,
        inv_paused: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cps: 10.0,
            running: false,
            status: "Ready".to_string(),
            running_flag,
            settings,
            inventory: "E".to_string(),
            hotkey,
            inv_paused,
        }
    }

    fn toggle_clicking(&mut self) {
        let new_running = !self.running_flag.load(Ordering::Relaxed);
        self.running_flag.store(new_running, Ordering::Relaxed);
        self.inv_paused.store(false, Ordering::Relaxed);

        self.running = new_running;
        self.status = if self.running {
            "Clics actifs"
        } else {
            "Arrêté"
        }
            .to_string();
    }

    fn parse_inventory_key(&self) -> Option<Key> {
        let s = self.inventory.trim();
        if s.eq_ignore_ascii_case("e") {
            Some(Key::KeyE)
        } else if s.eq_ignore_ascii_case("f4") {
            Some(Key::F4)
        } else {
            None
        }
    }
}

impl eframe::App for AutoclickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let flag_running = self.running_flag.load(Ordering::Relaxed);
        let inv_paused = self.inv_paused.load(Ordering::Relaxed);

        if flag_running != self.running {
            self.running = flag_running;
        }

        self.status = if self.running {
            "Active click".to_string()
        } else if inv_paused {
            "Paused (inventory)".to_string()
        } else {
            "Stopped".to_string()
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AutoClicker v2.0");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Clics/sec:");
                ui.add(egui::Slider::new(&mut self.cps, 1.0..=100.0));
                ui.label(format!("{:.1}", self.cps));
            });

            if let Ok(mut s) = self.settings.lock() {
                s.cps = self.cps.round().clamp(1.0, 100.0) as u32;
            }

            ui.separator();

            let button_text = if self.running { "STOP" } else { "START" };
            if ui.button(button_text).clicked() {
                self.toggle_clicking();
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Inventory key:");
                ui.text_edit_singleline(&mut self.inventory);

                if let Some(k) = self.parse_inventory_key() {
                    if let Ok(mut hk) = self.hotkey.lock() {
                        *hk = k;
                    }
                }
            });

            ui.label(&self.status);
            ui.label("Toggle: F4");
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn autoclick_once(cps: u32) {
    let delay_ms = Duration::from_millis(1000u64 / cps.max(1) as u64).max(Duration::from_millis(1));

    let _ = simulate(&EventType::ButtonPress(Button::Left));
    let _ = simulate(&EventType::ButtonRelease(Button::Left));
    thread::sleep(delay_ms);
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn relaunch_as_admin() {
    let exe_str = env::current_exe().unwrap().to_string_lossy().into_owned();

    let operation = to_wide("runas");
    let file = to_wide(&*exe_str);

    unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        );
    }
}

fn main() -> Result<(), eframe::Error> {
    // Auto elevation
    if unsafe { !IsUserAnAdmin().as_bool() } {
        relaunch_as_admin();
        return Ok(());
    }

    let running_flag = Arc::new(AtomicBool::new(false));
    let settings = Arc::new(Mutex::new(ClickSettings { cps: 13 }));
    let hotkey = Arc::new(Mutex::new(Key::KeyE));
    let inv_paused = Arc::new(AtomicBool::new(false));

    // Hotkey thread
    {
        let running_flag = Arc::clone(&running_flag);
        let hotkey = Arc::clone(&hotkey);
        let inv_paused = Arc::clone(&inv_paused);

        thread::spawn(move || {
            let _ = listen(move |event| {
                if let EventType::KeyRelease(key) = event.event_type {
                    if key == Key::F4 {
                        let new_val = !running_flag.load(Ordering::Relaxed);
                        running_flag.store(new_val, Ordering::Relaxed);
                        inv_paused.store(false, Ordering::Relaxed);
                        return;
                    }

                    let inventory_key =
                        hotkey.lock().map(|g| *g).unwrap_or(Key::KeyE);

                    if key == inventory_key {
                        let running = running_flag.load(Ordering::Relaxed);
                        let paused = inv_paused.load(Ordering::Relaxed);

                        if running {
                            running_flag.store(false, Ordering::Relaxed);
                            inv_paused.store(true, Ordering::Relaxed);
                        } else if paused {
                            running_flag.store(true, Ordering::Relaxed);
                            inv_paused.store(false, Ordering::Relaxed);
                        }
                    }
                }
            });
        });
    }

    // Click thread
    {
        let running_flag = Arc::clone(&running_flag);
        let settings = Arc::clone(&settings);

        thread::spawn(move || loop {
            if running_flag.load(Ordering::Relaxed) {
                let cps = settings.lock().map(|s| s.cps).unwrap_or(10);
                autoclick_once(cps);
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 350.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Golemian Autoclicker",
        options,
        Box::new({
            let running_flag_for_app = Arc::clone(&running_flag);
            let settings_for_app = Arc::clone(&settings);
            let hotkey_for_app = Arc::clone(&hotkey);
            let inv_paused_for_app = Arc::clone(&inv_paused);
            move |_cc| Ok(Box::new(AutoclickerApp::new(
                running_flag_for_app,
                settings_for_app,
                hotkey_for_app,
                inv_paused_for_app,
            )))
        }),
    )
}