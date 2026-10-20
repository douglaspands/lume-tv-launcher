//! Android key codes the launcher understands.

use crate::app::Key;

pub const KEYCODE_HOME: i32 = 3;
pub const KEYCODE_BACK: i32 = 4;
pub const KEYCODE_DPAD_UP: i32 = 19;
pub const KEYCODE_DPAD_DOWN: i32 = 20;
pub const KEYCODE_DPAD_LEFT: i32 = 21;
pub const KEYCODE_DPAD_RIGHT: i32 = 22;
pub const KEYCODE_DPAD_CENTER: i32 = 23;
pub const KEYCODE_ENTER: i32 = 66;
/// The Menu button of the remote and of a keyboard.
pub const KEYCODE_MENU: i32 = 82;
/// What the Menu button of the AIWA (KTC) remote sends; its firmware labels it "bookmark".
pub const KEYCODE_BOOKMARK: i32 = 174;
pub const KEYCODE_ESCAPE: i32 = 111;
pub const KEYCODE_NUMPAD_ENTER: i32 = 160;
/// "Select" on some remotes and game controllers.
pub const KEYCODE_BUTTON_A: i32 = 96;
pub const KEYCODE_BUTTON_B: i32 = 97;

/// Maps an Android `KeyEvent` key code to a launcher key.
pub fn from_android(code: i32) -> Option<Key> {
    Some(match code {
        KEYCODE_DPAD_UP => Key::Up,
        KEYCODE_DPAD_DOWN => Key::Down,
        KEYCODE_DPAD_LEFT => Key::Left,
        KEYCODE_DPAD_RIGHT => Key::Right,
        KEYCODE_DPAD_CENTER | KEYCODE_ENTER | KEYCODE_NUMPAD_ENTER | KEYCODE_BUTTON_A => Key::Ok,
        KEYCODE_BACK | KEYCODE_ESCAPE | KEYCODE_BUTTON_B => Key::Back,
        KEYCODE_HOME => Key::Home,
        KEYCODE_MENU | KEYCODE_BOOKMARK => Key::Menu,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_remote_keys_are_mapped() {
        assert_eq!(from_android(19), Some(Key::Up));
        assert_eq!(from_android(20), Some(Key::Down));
        assert_eq!(from_android(21), Some(Key::Left));
        assert_eq!(from_android(22), Some(Key::Right));
        assert_eq!(from_android(23), Some(Key::Ok), "the center of the D-pad");
        assert_eq!(from_android(66), Some(Key::Ok), "Enter");
        assert_eq!(from_android(160), Some(Key::Ok), "the keypad's Enter");
        assert_eq!(from_android(4), Some(Key::Back));
        assert_eq!(from_android(111), Some(Key::Back), "Escape on a keyboard");
        assert_eq!(from_android(3), Some(Key::Home));
        assert_eq!(from_android(82), Some(Key::Menu));
        assert_eq!(from_android(174), Some(Key::Menu), "the AIWA remote's Menu");
    }

    #[test]
    fn other_keys_are_ignored() {
        for code in [0, 7, 24, 25, 85, 126, 164, 9999, -1] {
            assert_eq!(from_android(code), None, "{code}");
        }
    }
}
