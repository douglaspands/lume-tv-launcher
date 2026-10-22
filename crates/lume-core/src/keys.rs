//! Android remote control keys, keycode mappings, and persistent configuration.

use crate::storage::{self, Kv, StorageError};
use std::path::Path;

/// Remote control keys.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    /// OK, Enter or the center of the D-pad.
    Ok = 4,
    Back = 5,
    Home = 6,
    /// Opens the context menu of the focused item, where it has one.
    Menu = 7,
}

impl Key {
    pub const COUNT: usize = 8;
    pub const ALL: [Key; Key::COUNT] = [
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::Ok,
        Key::Back,
        Key::Home,
        Key::Menu,
    ];

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Key::Up => "up",
            Key::Down => "down",
            Key::Left => "left",
            Key::Right => "right",
            Key::Ok => "ok",
            Key::Back => "back",
            Key::Home => "home",
            Key::Menu => "menu",
        }
    }

    pub fn parse_name(name: &str) -> Option<Key> {
        match name.trim().to_ascii_lowercase().as_str() {
            "up" => Some(Key::Up),
            "down" => Some(Key::Down),
            "left" => Some(Key::Left),
            "right" => Some(Key::Right),
            "ok" | "enter" => Some(Key::Ok),
            "back" => Some(Key::Back),
            "home" => Some(Key::Home),
            "menu" => Some(Key::Menu),
            _ => None,
        }
    }

    pub fn default_codes(self) -> &'static [i32] {
        match self {
            Key::Up => DEFAULT_UP,
            Key::Down => DEFAULT_DOWN,
            Key::Left => DEFAULT_LEFT,
            Key::Right => DEFAULT_RIGHT,
            Key::Ok => DEFAULT_OK,
            Key::Back => DEFAULT_BACK,
            Key::Home => DEFAULT_HOME,
            Key::Menu => DEFAULT_MENU,
        }
    }
}

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
/// "Select" on some remotes and game controllers.
pub const KEYCODE_BUTTON_A: i32 = 96;
pub const KEYCODE_BUTTON_B: i32 = 97;
pub const KEYCODE_ESCAPE: i32 = 111;
pub const KEYCODE_NUMPAD_ENTER: i32 = 160;
/// What the Menu button of the AIWA (KTC) remote sends; its firmware labels it "bookmark".
pub const KEYCODE_BOOKMARK: i32 = 174;

include!(concat!(env!("OUT_DIR"), "/keys_generated.rs"));

/// Maps an Android `KeyEvent` key code to a launcher key using default mappings.
pub fn from_android(code: i32) -> Option<Key> {
    Some(match code {
        c if DEFAULT_UP.contains(&c) => Key::Up,
        c if DEFAULT_DOWN.contains(&c) => Key::Down,
        c if DEFAULT_LEFT.contains(&c) => Key::Left,
        c if DEFAULT_RIGHT.contains(&c) => Key::Right,
        c if DEFAULT_OK.contains(&c) => Key::Ok,
        c if DEFAULT_BACK.contains(&c) => Key::Back,
        c if DEFAULT_HOME.contains(&c) => Key::Home,
        c if DEFAULT_MENU.contains(&c) => Key::Menu,
        _ => return None,
    })
}

const FAST_TABLE_SIZE: usize = 320;
const VERSION: u32 = 1;

/// In-memory remote control key mappings with fast O(1) translation.
#[derive(Clone, Debug)]
pub struct KeyMap {
    /// Active keycodes per logical Key, indexed by `key.index()` (0..8).
    bindings: [Vec<i32>; Key::COUNT],
    /// O(1) lookup table for Android keycodes in 0..320 (320 bytes inline on the stack).
    fast_table: [Option<Key>; FAST_TABLE_SIZE],
    /// Fallback list only for custom keycodes outside 0..320.
    overflow: Vec<(i32, Key)>,
}

impl PartialEq for KeyMap {
    fn eq(&self, other: &Self) -> bool {
        self.bindings == other.bindings
    }
}

impl Eq for KeyMap {}

impl Default for KeyMap {
    fn default() -> Self {
        let bindings = [
            Key::Up.default_codes().to_vec(),
            Key::Down.default_codes().to_vec(),
            Key::Left.default_codes().to_vec(),
            Key::Right.default_codes().to_vec(),
            Key::Ok.default_codes().to_vec(),
            Key::Back.default_codes().to_vec(),
            Key::Home.default_codes().to_vec(),
            Key::Menu.default_codes().to_vec(),
        ];
        let mut map = KeyMap {
            bindings,
            fast_table: [None; FAST_TABLE_SIZE],
            overflow: Vec::new(),
        };
        map.rebuild_lookup();
        map
    }
}

impl KeyMap {
    fn rebuild_lookup(&mut self) {
        self.fast_table = [None; FAST_TABLE_SIZE];
        self.overflow.clear();
        for key in Key::ALL {
            for &code in &self.bindings[key.index()] {
                if code >= 0 && (code as usize) < FAST_TABLE_SIZE {
                    if self.fast_table[code as usize].is_none() {
                        self.fast_table[code as usize] = Some(key);
                    }
                } else if !self.overflow.iter().any(|&(c, _)| c == code) {
                    self.overflow.push((code, key));
                }
            }
        }
    }

    /// Maps an Android keycode to a launcher Key.
    #[inline]
    pub fn from_android(&self, code: i32) -> Option<Key> {
        if code >= 0 && (code as usize) < FAST_TABLE_SIZE {
            return self.fast_table[code as usize];
        }
        self.overflow
            .iter()
            .find_map(|&(c, key)| (c == code).then_some(key))
    }

    /// Returns active keycodes for `key`.
    pub fn codes(&self, key: Key) -> &[i32] {
        &self.bindings[key.index()]
    }

    /// Sets active keycodes for `key`, deduplicating and removing conflicts from other keys.
    pub fn set(&mut self, key: Key, codes: Vec<i32>) {
        let mut deduped: Vec<i32> = Vec::with_capacity(codes.len());
        for c in codes {
            if !deduped.contains(&c) {
                deduped.push(c);
            }
        }
        for other in Key::ALL {
            if other != key {
                self.bindings[other.index()].retain(|c| !deduped.contains(c));
            }
        }
        self.bindings[key.index()] = deduped;
        self.rebuild_lookup();
    }

    /// Clears keycodes for `key`, disabling it.
    pub fn clear(&mut self, key: Key) {
        self.set(key, Vec::new());
    }

    /// Resets `key` to its default keycodes.
    pub fn reset_key(&mut self, key: Key) {
        self.set(key, key.default_codes().to_vec());
    }

    /// Resets all keys to default keycodes.
    pub fn reset(&mut self) {
        *self = KeyMap::default();
    }

    /// Checks if `key` matches its default keycodes.
    pub fn is_default(&self, key: Key) -> bool {
        self.codes(key) == key.default_codes()
    }

    /// Checks if all keys match their default keycodes.
    pub fn is_all_default(&self) -> bool {
        Key::ALL.into_iter().all(|k| self.is_default(k))
    }

    fn to_kv(&self) -> Kv {
        let mut kv = Kv::with_capacity(Key::COUNT);
        for key in Key::ALL {
            let codes: Vec<String> = self.codes(key).iter().map(i32::to_string).collect();
            kv.set_list(key.as_str(), codes.iter().map(String::as_str));
        }
        kv
    }

    /// Parses a KeyMap from Kv, returning the map and whether healing/saving is needed.
    pub fn from_kv(kv: &Kv) -> (KeyMap, bool) {
        let mut map = KeyMap::default();
        let mut needs_save = false;

        for key in Key::ALL {
            match kv.get(key.as_str()) {
                None => {
                    needs_save = true;
                }
                Some(raw) => {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        map.bindings[key.index()] = Vec::new();
                    } else {
                        let mut parsed = Vec::new();
                        let mut ok = true;
                        for item in trimmed.split(',') {
                            let item_trimmed = item.trim();
                            if item_trimmed.is_empty() {
                                ok = false;
                                break;
                            }
                            match item_trimmed.parse::<i32>() {
                                Ok(c) => {
                                    if !parsed.contains(&c) {
                                        parsed.push(c);
                                    }
                                }
                                Err(_) => {
                                    ok = false;
                                    break;
                                }
                            }
                        }

                        if ok {
                            map.bindings[key.index()] = parsed;
                        } else {
                            log::warn!("ignoring invalid keycodes for {}: {:?}", key.as_str(), raw);
                            needs_save = true;
                        }
                    }
                }
            }
        }

        map.rebuild_lookup();
        (map, needs_save)
    }

    /// Saves the key mapping to disk.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        storage::save(path, VERSION, &self.to_kv())
    }

    /// Loads `keys.v1` from `path`. If the file is missing, corrupt, or had missing/invalid entries,
    /// repairs it in memory and atomically writes the clean `keys.v1` to disk.
    pub fn load_or_heal(path: &Path) -> KeyMap {
        match storage::load(path, VERSION) {
            Ok(kv) => {
                let (map, needs_save) = KeyMap::from_kv(&kv);
                if needs_save {
                    if let Err(e) = map.save(path) {
                        log::warn!("cannot write repaired {}: {e}", path.display());
                    }
                }
                map
            }
            Err(StorageError::Missing) => {
                let map = KeyMap::default();
                if let Err(e) = map.save(path) {
                    log::warn!("cannot write initial {}: {e}", path.display());
                }
                map
            }
            Err(e) => {
                log::warn!("healing {}: {e}", path.display());
                let map = KeyMap::default();
                if let Err(err) = map.save(path) {
                    log::warn!("cannot write healed {}: {err}", path.display());
                }
                map
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lume-keys-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn size_of_option_key_is_one_byte() {
        assert_eq!(std::mem::size_of::<Option<Key>>(), 1);
    }

    #[test]
    fn default_mappings_from_android() {
        let default_map = KeyMap::default();

        for &(code, expected) in &[
            (19, Some(Key::Up)),
            (20, Some(Key::Down)),
            (21, Some(Key::Left)),
            (22, Some(Key::Right)),
            (23, Some(Key::Ok)),
            (66, Some(Key::Ok)),
            (160, Some(Key::Ok)),
            (96, Some(Key::Ok)),
            (4, Some(Key::Back)),
            (111, Some(Key::Back)),
            (97, Some(Key::Back)),
            (3, Some(Key::Home)),
            (82, Some(Key::Menu)),
            (174, Some(Key::Menu)),
        ] {
            assert_eq!(from_android(code), expected, "code {code}");
            assert_eq!(
                default_map.from_android(code),
                expected,
                "default_map code {code}"
            );
        }

        for code in [0, 7, 24, 25, 85, 126, 164, 9999, -1] {
            assert_eq!(from_android(code), None, "unmapped code {code}");
            assert_eq!(
                default_map.from_android(code),
                None,
                "default_map unmapped code {code}"
            );
        }
    }

    #[test]
    fn overflow_custom_keycodes() {
        let mut map = KeyMap::default();
        map.set(Key::Menu, vec![500, -10]);
        assert_eq!(map.from_android(500), Some(Key::Menu));
        assert_eq!(map.from_android(-10), Some(Key::Menu));
        assert_eq!(map.from_android(82), None);
        assert_eq!(map.from_android(174), None);
    }

    #[test]
    fn auto_create_when_missing() {
        let d = dir("missing");
        let path = d.join("keys.v1");
        assert!(!path.exists());

        let map = KeyMap::load_or_heal(&path);
        assert!(path.exists());
        assert!(map.is_all_default());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("lume=1\n"));
        assert!(content.ends_with("#end\n"));
    }

    #[test]
    fn load_existing_valid_file_without_rewriting() {
        let d = dir("valid");
        let path = d.join("keys.v1");
        let custom_content = "lume=1\nup=19\ndown=20\nleft=21\nright=22\nok=23,66,160,96\nback=4,111,97\nhome=3\nmenu=165\n#end\n";
        fs::write(&path, custom_content).unwrap();

        let map = KeyMap::load_or_heal(&path);
        assert_eq!(map.codes(Key::Menu), &[165]);
        assert_eq!(map.from_android(165), Some(Key::Menu));
        assert_eq!(map.from_android(82), None);

        let after_content = fs::read_to_string(&path).unwrap();
        assert_eq!(after_content, custom_content, "file was not rewritten");
    }

    #[test]
    fn multiple_keycodes_for_single_key() {
        let d = dir("multi_key");
        let path = d.join("keys.v1");
        let custom_content = "lume=1\nup=19\ndown=20\nleft=21\nright=22\nok=23,66,160,96\nback=4,111,97\nhome=3\nmenu=82,174,165\n#end\n";
        fs::write(&path, custom_content).unwrap();

        let map = KeyMap::load_or_heal(&path);
        assert_eq!(map.codes(Key::Menu), &[82, 174, 165]);
        assert_eq!(map.from_android(82), Some(Key::Menu));
        assert_eq!(map.from_android(174), Some(Key::Menu));
        assert_eq!(map.from_android(165), Some(Key::Menu));
    }

    #[test]
    fn empty_value_disables_key() {
        let d = dir("disabled");
        let path = d.join("keys.v1");
        let content = "lume=1\nup=19\ndown=20\nleft=21\nright=22\nok=23,66,160,96\nback=4,111,97\nhome=3\nmenu=\n#end\n";
        fs::write(&path, content).unwrap();

        let map = KeyMap::load_or_heal(&path);
        assert!(map.codes(Key::Menu).is_empty());
        assert_eq!(map.from_android(82), None);
        assert_eq!(map.from_android(174), None);

        let after_content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            after_content, content,
            "file was not rewritten for disabled key"
        );
    }

    #[test]
    fn healing_corrupted_file() {
        let d = dir("corrupt");
        let path = d.join("keys.v1");
        fs::write(&path, "lume=1\nup=19\n").unwrap(); // missing #end

        let map = KeyMap::load_or_heal(&path);
        assert!(map.is_all_default());

        let healed = fs::read_to_string(&path).unwrap();
        assert!(healed.starts_with("lume=1\n"));
        assert!(healed.ends_with("#end\n"));
        assert!(healed.contains("menu=82,174"));
    }

    #[test]
    fn partial_healing_invalid_token_or_missing_key() {
        let d = dir("partial_heal");
        let path = d.join("keys.v1");
        // menu has custom 165, ok has invalid 23,abc, home is missing
        let partial_content =
            "lume=1\nup=19\ndown=20\nleft=21\nright=22\nok=23,abc\nback=4,111,97\nmenu=165\n#end\n";
        fs::write(&path, partial_content).unwrap();

        let map = KeyMap::load_or_heal(&path);
        assert_eq!(map.codes(Key::Menu), &[165], "custom preserved");
        assert_eq!(
            map.codes(Key::Ok),
            Key::Ok.default_codes(),
            "repaired ok to default"
        );
        assert_eq!(
            map.codes(Key::Home),
            Key::Home.default_codes(),
            "repaired missing home to default"
        );

        let healed = fs::read_to_string(&path).unwrap();
        assert!(healed.contains("menu=165"));
        assert!(healed.contains("ok=23,66,160,96"));
        assert!(healed.contains("home=3"));
        assert!(healed.ends_with("#end\n"));
    }

    #[test]
    fn editing_and_reset_methods() {
        let mut map = KeyMap::default();
        assert!(map.is_all_default());

        // set with deduplication and removing conflict from other keys
        map.set(Key::Menu, vec![19, 19, 200]);
        assert_eq!(map.codes(Key::Menu), &[19, 200]);
        assert!(
            !map.codes(Key::Up).contains(&19),
            "19 removed from Up to avoid conflict"
        );
        assert_eq!(map.from_android(19), Some(Key::Menu));
        assert_eq!(map.from_android(200), Some(Key::Menu));
        assert!(!map.is_default(Key::Menu));
        assert!(!map.is_default(Key::Up));
        assert!(!map.is_all_default());

        // reset_key
        map.reset_key(Key::Up);
        assert!(map.is_default(Key::Up));
        assert!(
            !map.codes(Key::Menu).contains(&19),
            "19 removed from Menu when Up reset"
        );
        assert_eq!(map.from_android(19), Some(Key::Up));

        // clear
        map.clear(Key::Back);
        assert!(map.codes(Key::Back).is_empty());
        assert_eq!(map.from_android(4), None);
        assert!(!map.is_default(Key::Back));

        // reset
        map.reset();
        assert!(map.is_all_default());
        assert_eq!(map.from_android(4), Some(Key::Back));
        assert_eq!(map.from_android(82), Some(Key::Menu));
    }
}
