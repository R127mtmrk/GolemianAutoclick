use rdev::{EventType, Key};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyBindingTarget {
    Inventory,
    Toggle,
}

pub(crate) struct SharedState {
    pub(crate) cps: u32,
    pub(crate) running: bool,
    pub(crate) inv_paused: bool,
    pub(crate) inventory_key: Key,
    pub(crate) toggle_key: Key,
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
        "Administrator mode is required for this app. Please accept the UAC prompt on launch.".to_string()
    }
}

pub(crate) fn key_to_label(key: Key) -> String {
    match key {
        Key::Alt => "Alt", Key::AltGr => "Alt Gr", Key::BackQuote => "`", Key::BackSlash => "\\", Key::Backspace => "Backspace", Key::CapsLock => "Caps Lock", Key::Comma => ",",
        Key::ControlLeft => "Left Ctrl", Key::ControlRight => "Right Ctrl", Key::Delete => "Delete", Key::Dot => ".", Key::DownArrow => "Down Arrow", Key::End => "End",
        Key::Escape => "Escape", Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4", Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12", Key::Home => "Home", Key::Insert => "Insert", Key::KeyA => "A", Key::KeyB => "B",
        Key::KeyC => "C", Key::KeyD => "D", Key::KeyE => "E", Key::KeyF => "F", Key::KeyG => "G", Key::KeyH => "H", Key::KeyI => "I", Key::KeyJ => "J", Key::KeyK => "K",
        Key::KeyL => "L", Key::KeyM => "M", Key::KeyN => "N", Key::KeyO => "O", Key::KeyP => "P", Key::KeyQ => "Q", Key::KeyR => "R", Key::KeyS => "S", Key::KeyT => "T",
        Key::KeyU => "U", Key::KeyV => "V", Key::KeyW => "W", Key::KeyX => "X", Key::KeyY => "Y", Key::KeyZ => "Z", Key::LeftArrow => "Left Arrow", Key::LeftBracket => "[",
        Key::MetaLeft => "Left Win", Key::MetaRight => "Right Win", Key::Minus => "-", Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2", Key::Num3 => "3", Key::Num4 => "4",
        Key::Num5 => "5", Key::Num6 => "6", Key::Num7 => "7", Key::Num8 => "8", Key::Num9 => "9", Key::NumLock => "Num Lock", Key::PageDown => "Page Down", Key::PageUp => "Page Up",
        Key::Pause => "Pause", Key::PrintScreen => "Print Screen", Key::Quote => "'", Key::Return => "Enter", Key::RightArrow => "Right Arrow", Key::RightBracket => "]",
        Key::ScrollLock => "Scroll Lock", Key::SemiColon => ";", Key::ShiftLeft => "Left Shift", Key::ShiftRight => "Right Shift", Key::Slash => "/", Key::Space => "Space",
        Key::Tab => "Tab", Key::UpArrow => "Up Arrow", Key::Kp0 => "Numpad 0", Key::Kp1 => "Numpad 1", Key::Kp2 => "Numpad 2", Key::Kp3 => "Numpad 3", Key::Kp4 => "Numpad 4",
        Key::Kp5 => "Numpad 5", Key::Kp6 => "Numpad 6", Key::Kp7 => "Numpad 7", Key::Kp8 => "Numpad 8", Key::Kp9 => "Numpad 9", Key::KpDelete => "Numpad .", Key::KpDivide => "Numpad /",
        Key::KpMinus => "Numpad -", Key::KpMultiply => "Numpad *", Key::KpPlus => "Numpad +", Key::KpReturn => "Numpad Enter", other => return format!("{other:?}"),
    }.to_string()
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

pub(crate) fn extract_relevant_key(event_type: EventType) -> Option<Key> {
    match event_type {
        EventType::KeyRelease(key) => Some(key),
        _ => None,
    }
}

pub(crate) fn set_notice(inner: &mut SharedState, notice: impl Into<String>) {
    inner.notice = notice.into();
}

fn is_binding_conflict(inner: &SharedState, target: KeyBindingTarget, key: Key) -> bool {
    match target {
        KeyBindingTarget::Inventory => key == inner.toggle_key,
        KeyBindingTarget::Toggle => key == inner.inventory_key,
    }
}

pub(crate) fn apply_pending_bind(inner: &mut SharedState, key: Key) -> bool {
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

pub(crate) fn handle_key_release(inner: &mut SharedState, key: Key) -> bool {
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

