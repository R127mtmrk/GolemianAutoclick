use serde::Serialize;

// Représente une touche par son Virtual Key code Windows (u32).
// Cela évite toute dépendance à rdev et tout hook souris.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotKey(pub(crate) u32);

impl HotKey {
    pub(crate) const ESCAPE: HotKey = HotKey(0x1B);
    pub(crate) const KEY_E:  HotKey = HotKey(0x45);
    pub(crate) const F4:     HotKey = HotKey(0x73);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyBindingTarget {
    Inventory,
    Toggle,
}

pub(crate) struct SharedState {
    pub(crate) cps: u32,
    pub(crate) running: bool,
    pub(crate) inv_paused: bool,
    pub(crate) inventory_key: HotKey,
    pub(crate) toggle_key: HotKey,
    pub(crate) pending_bind: Option<KeyBindingTarget>,
    pub(crate) notice: String,
    pub(crate) is_elevated: bool,
}

#[derive(Serialize, Clone)]
pub(crate) struct UiState {
    pub(crate) cps: u32,
    pub(crate) running: bool,
    pub(crate) inv_paused: bool,
    pub(crate) status: String,
    pub(crate) inventory_key: String,
    pub(crate) toggle_key: String,
    pub(crate) pending_bind: Option<String>,
    pub(crate) notice: String,
    pub(crate) is_elevated: bool,
}

pub(crate) fn default_notice(is_elevated: bool) -> String {
    if is_elevated {
        "You can assign any keyboard key.".to_string()
    } else {
        "Administrator mode is required for this app. Please accept the UAC prompt on launch."
            .to_string()
    }
}

/// Convertit un Virtual Key code en label lisible.
pub(crate) fn key_to_label(key: HotKey) -> String {
    match key.0 {
        0x08 => "Backspace", 0x09 => "Tab", 0x0D => "Enter", 0x1B => "Escape",
        0x20 => "Space", 0x21 => "Page Up", 0x22 => "Page Down", 0x23 => "End",
        0x24 => "Home", 0x25 => "Left Arrow", 0x26 => "Up Arrow", 0x27 => "Right Arrow",
        0x28 => "Down Arrow", 0x2C => "Print Screen", 0x2D => "Insert", 0x2E => "Delete",
        0x30 => "0", 0x31 => "1", 0x32 => "2", 0x33 => "3", 0x34 => "4",
        0x35 => "5", 0x36 => "6", 0x37 => "7", 0x38 => "8", 0x39 => "9",
        0x41 => "A", 0x42 => "B", 0x43 => "C", 0x44 => "D", 0x45 => "E",
        0x46 => "F", 0x47 => "G", 0x48 => "H", 0x49 => "I", 0x4A => "J",
        0x4B => "K", 0x4C => "L", 0x4D => "M", 0x4E => "N", 0x4F => "O",
        0x50 => "P", 0x51 => "Q", 0x52 => "R", 0x53 => "S", 0x54 => "T",
        0x55 => "U", 0x56 => "V", 0x57 => "W", 0x58 => "X", 0x59 => "Y",
        0x5A => "Z",
        0x5B => "Left Win", 0x5C => "Right Win",
        0x60 => "Numpad 0", 0x61 => "Numpad 1", 0x62 => "Numpad 2", 0x63 => "Numpad 3",
        0x64 => "Numpad 4", 0x65 => "Numpad 5", 0x66 => "Numpad 6", 0x67 => "Numpad 7",
        0x68 => "Numpad 8", 0x69 => "Numpad 9", 0x6A => "Numpad *", 0x6B => "Numpad +",
        0x6D => "Numpad -", 0x6E => "Numpad .", 0x6F => "Numpad /",
        0x70 => "F1",  0x71 => "F2",  0x72 => "F3",  0x73 => "F4",
        0x74 => "F5",  0x75 => "F6",  0x76 => "F7",  0x77 => "F8",
        0x78 => "F9",  0x79 => "F10", 0x7A => "F11", 0x7B => "F12",
        0x90 => "Num Lock", 0x91 => "Scroll Lock",
        0xA0 => "Left Shift", 0xA1 => "Right Shift",
        0xA2 => "Left Ctrl", 0xA3 => "Right Ctrl",
        0xA4 => "Alt", 0xA5 => "Alt Gr",
        0xBA => ";", 0xBB => "=", 0xBC => ",", 0xBD => "-", 0xBE => ".",
        0xBF => "/", 0xC0 => "`", 0xDB => "[", 0xDC => "\\", 0xDD => "]", 0xDE => "'",
        0x13 => "Pause", 0x14 => "Caps Lock",
        vk => return format!("VK(0x{vk:02X})"),
    }
    .to_string()
}

pub(crate) fn target_to_ui_label(target: KeyBindingTarget) -> &'static str {
    match target {
        KeyBindingTarget::Inventory => "Inventory pause",
        KeyBindingTarget::Toggle => "Toggle autoclick",
    }
}

pub(crate) fn to_ui_state(inner: &SharedState) -> UiState {
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

pub(crate) fn set_notice(inner: &mut SharedState, notice: impl Into<String>) {
    inner.notice = notice.into();
}

fn is_binding_conflict(inner: &SharedState, target: KeyBindingTarget, key: HotKey) -> bool {
    match target {
        KeyBindingTarget::Inventory => key == inner.toggle_key,
        KeyBindingTarget::Toggle => key == inner.inventory_key,
    }
}

pub(crate) fn apply_pending_bind(inner: &mut SharedState, key: HotKey) -> bool {
    let Some(target) = inner.pending_bind else {
        return false;
    };

    if key == HotKey::ESCAPE {
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
        format!(
            "{} is now set to {}.",
            target_to_ui_label(target),
            key_to_label(key)
        ),
    );
    true
}

pub(crate) fn handle_key_release(inner: &mut SharedState, key: HotKey) -> bool {
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
