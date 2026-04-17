use crate::app_logic::{key_to_label, HotKey};

use crate::app_logic::{
    apply_pending_bind, handle_key_release, KeyBindingTarget, SharedState,
};

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
}