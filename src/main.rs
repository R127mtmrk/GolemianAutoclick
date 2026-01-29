use eframe::egui;
use rdev::{listen, simulate, Button, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::Win32::UI::Shell::IsUserAnAdmin;
use native_dialog::{MessageDialog, MessageType};

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

    fn is_admin() -> bool {
        unsafe { IsUserAnAdmin().as_bool() }
    }

    fn toggle_clicking(&mut self) {
        let new_running = !self.running_flag.load(Ordering::Relaxed);
        self.running_flag.store(new_running, Ordering::Relaxed);

        // If user manually toggles, clear inventory-pause state
        self.inv_paused.store(false, Ordering::Relaxed);

        self.running = new_running;
        self.status = if self.running { "Clics actifs" } else { "Arrêté" }.to_string();
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

        // Keep status consistent with both flags
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

            // Push latest settings to worker thread
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

            if self.parse_inventory_key().is_none() {
                ui.label("Unknown key. Try: E, I, or F4");
            }

            ui.label(&self.status);
            ui.label("Toggle: F4");
            ui.label("Inventory pause/resume: E (press while active to stop, press again to resume)");
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn autoclick_once(cps: u32) {
    let cps = cps.max(1);
    let delay_ms = (1000u64 / cps as u64).max(1);
    let delay = Duration::from_millis(delay_ms);

    let _ = simulate(&EventType::ButtonPress(Button::Left));
    let _ = simulate(&EventType::ButtonRelease(Button::Left));
    thread::sleep(delay);
}

fn main() -> Result<(), eframe::Error> {
    // Require elevation (safe: detect + refuse to run)
    if !AutoclickerApp::is_admin() {
        let _ = MessageDialog::new()
            .set_type(MessageType::Error)
            .set_title("Administrator required")
            .set_text("Please run this program as Administrator.")
            .show_alert();

        // Return an error so the app clearly fails to start.
        // (eframe::Error doesn't have a simple constructor, so we just stop cleanly.)
        return Ok(());
    }

    let running_flag = Arc::new(AtomicBool::new(false));
    let settings = Arc::new(Mutex::new(ClickSettings { cps: 13 }));

    // Inventory key (default: E)
    let hotkey = Arc::new(Mutex::new(Key::KeyE));

    // Tracks whether we are currently paused by inventory key
    let inv_paused = Arc::new(AtomicBool::new(false));

    // Global hotkey listener:
    // - F4 toggles normally (start/stop)
    // - Inventory key (E) acts as: if running => stop and mark paused; if paused => resume
    {
        let running_flag = Arc::clone(&running_flag);
        let hotkey = Arc::clone(&hotkey);
        let inv_paused = Arc::clone(&inv_paused);

        thread::spawn(move || {
            let _ = listen(move |event| {
                if let EventType::KeyRelease(key) = event.event_type {
                    // F4: normal toggle, also clears inventory pause
                    if key == Key::F4 {
                        let new_val = !running_flag.load(Ordering::Relaxed);
                        running_flag.store(new_val, Ordering::Relaxed);
                        inv_paused.store(false, Ordering::Relaxed);
                        return;
                    }

                    // Inventory key behavior
                    let inventory_key = hotkey
                            .lock()
                            .map(|g| *g)
                            .unwrap_or(Key::KeyE);

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

    // Click worker thread
    {
        let running_flag = Arc::clone(&running_flag);
        let settings = Arc::clone(&settings);
        thread::spawn(move || loop {
            if running_flag.load(Ordering::Relaxed) {
                let cps = match settings.lock() {
                    Ok(s) => s.cps.max(1),
                    Err(_) => 10,
                };
                autoclick_once(cps);
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 350.0]),
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