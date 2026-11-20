//! The keyboard shortcuts of the player: which keys run which command.
//!
//! [`BINDINGS`] lists every shortcut with the keys it has from the start.
//! What the user changed in Preferences > Hotkeys is kept in the ini file
//! (`[Hotkeys]`, one line for each shortcut that is not as it came) and
//! read into a [`Keymap`], which the window code asks when a key is pressed.

use crate::menu::{MenuAction, MenuItem};
use popamp_core::Command;
use winit::keyboard::KeyCode as K;

/// Where a shortcut works: in every window, or only in one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Playlist,
    Equalizer,
}

/// A key and the Ctrl, Shift and Alt held with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub code: K,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// The keys that can be part of a shortcut, and how they are written.
const NAMES: &[(K, &str)] = &[
    (K::KeyA, "A"),
    (K::KeyB, "B"),
    (K::KeyC, "C"),
    (K::KeyD, "D"),
    (K::KeyE, "E"),
    (K::KeyF, "F"),
    (K::KeyG, "G"),
    (K::KeyH, "H"),
    (K::KeyI, "I"),
    (K::KeyJ, "J"),
    (K::KeyK, "K"),
    (K::KeyL, "L"),
    (K::KeyM, "M"),
    (K::KeyN, "N"),
    (K::KeyO, "O"),
    (K::KeyP, "P"),
    (K::KeyQ, "Q"),
    (K::KeyR, "R"),
    (K::KeyS, "S"),
    (K::KeyT, "T"),
    (K::KeyU, "U"),
    (K::KeyV, "V"),
    (K::KeyW, "W"),
    (K::KeyX, "X"),
    (K::KeyY, "Y"),
    (K::KeyZ, "Z"),
    (K::Digit0, "0"),
    (K::Digit1, "1"),
    (K::Digit2, "2"),
    (K::Digit3, "3"),
    (K::Digit4, "4"),
    (K::Digit5, "5"),
    (K::Digit6, "6"),
    (K::Digit7, "7"),
    (K::Digit8, "8"),
    (K::Digit9, "9"),
    (K::F1, "F1"),
    (K::F2, "F2"),
    (K::F3, "F3"),
    (K::F4, "F4"),
    (K::F5, "F5"),
    (K::F6, "F6"),
    (K::F7, "F7"),
    (K::F8, "F8"),
    (K::F9, "F9"),
    (K::F10, "F10"),
    (K::F11, "F11"),
    (K::F12, "F12"),
    (K::ArrowUp, "Up"),
    (K::ArrowDown, "Down"),
    (K::ArrowLeft, "Left"),
    (K::ArrowRight, "Right"),
    (K::Enter, "Enter"),
    (K::NumpadEnter, "NumEnter"),
    (K::Delete, "Delete"),
    (K::Insert, "Insert"),
    (K::Home, "Home"),
    (K::End, "End"),
    (K::PageUp, "PageUp"),
    (K::PageDown, "PageDown"),
    (K::Tab, "Tab"),
    (K::Space, "Space"),
    (K::Backspace, "Backspace"),
    (K::Backquote, "`"),
    (K::Minus, "-"),
    (K::Equal, "="),
    (K::Comma, ","),
    (K::Period, "."),
    (K::Slash, "/"),
    (K::Semicolon, ";"),
    (K::Quote, "'"),
    (K::BracketLeft, "["),
    (K::BracketRight, "]"),
    (K::Backslash, "\\"),
    (K::Numpad0, "Num0"),
    (K::Numpad1, "Num1"),
    (K::Numpad2, "Num2"),
    (K::Numpad3, "Num3"),
    (K::Numpad4, "Num4"),
    (K::Numpad5, "Num5"),
    (K::Numpad6, "Num6"),
    (K::Numpad7, "Num7"),
    (K::Numpad8, "Num8"),
    (K::Numpad9, "Num9"),
    (K::NumpadDecimal, "Num."),
    (K::NumpadAdd, "Num+"),
    (K::NumpadSubtract, "Num-"),
    (K::NumpadMultiply, "Num*"),
    (K::NumpadDivide, "Num/"),
];

impl Chord {
    /// The chord of a key press, when the key can be part of a shortcut.
    pub fn new(code: K, ctrl: bool, shift: bool, alt: bool) -> Option<Chord> {
        NAMES.iter().any(|n| n.0 == code).then_some(Chord { code, ctrl, shift, alt })
    }

    /// `Ctrl+Shift+R`, `Num4`: the modifiers, then the key.
    pub fn parse(text: &str) -> Option<Chord> {
        let mut rest = text.trim();
        let (mut ctrl, mut shift, mut alt) = (false, false, false);
        loop {
            let cut = |prefix: &str| rest.get(..prefix.len()).filter(|h| h.eq_ignore_ascii_case(prefix)).map(|_| &rest[prefix.len()..]);
            if let Some(r) = cut("Ctrl+") {
                (ctrl, rest) = (true, r);
            } else if let Some(r) = cut("Shift+") {
                (shift, rest) = (true, r);
            } else if let Some(r) = cut("Alt+") {
                (alt, rest) = (true, r);
            } else {
                break;
            }
        }
        let code = NAMES.iter().find(|n| n.1.eq_ignore_ascii_case(rest))?.0;
        Some(Chord { code, ctrl, shift, alt })
    }

    pub fn text(&self) -> String {
        let key = NAMES.iter().find(|n| n.0 == self.code).map_or("?", |n| n.1);
        let mut out = String::new();
        for (on, name) in [(self.ctrl, "Ctrl+"), (self.shift, "Shift+"), (self.alt, "Alt+")] {
            if on {
                out.push_str(name);
            }
        }
        out + key
    }
}

/// One shortcut: what it does, where it works, and the keys it comes with.
pub struct Binding {
    /// How it is kept in the ini file.
    pub id: &'static str,
    /// The heading it is listed under.
    #[allow(dead_code)]
    pub group: &'static str,
    pub label: &'static str,
    pub scope: Scope,
    pub command: Command,
    pub keys: &'static [&'static str],
}

const fn b(id: &'static str, group: &'static str, label: &'static str, scope: Scope, command: Command, keys: &'static [&'static str]) -> Binding {
    Binding { id, group, label, scope, command, keys }
}

use Command as C;
use Scope::{Equalizer, Global, Playlist};

/// Every shortcut, with its keys from the start. The ones of the whole
/// player come first: a window's own are asked before them.
pub const BINDINGS: &[Binding] = &[
    b("previous", "Playback", "Previous track", Global, C::Previous, &["Z", "Num4"]),
    b("play", "Playback", "Play", Global, C::Play, &["X", "Num5"]),
    b("pause", "Playback", "Pause", Global, C::Pause, &["C"]),
    b("stop", "Playback", "Stop", Global, C::Stop, &["V"]),
    b("stop_fade", "Playback", "Stop with fade-out", Global, C::StopFadeout, &["Shift+V"]),
    b("next", "Playback", "Next track", Global, C::Next, &["B", "Num6"]),
    b("back5", "Playback", "Rewind 5 seconds", Global, C::Back5s, &["Left", "Num7"]),
    b("fwd5", "Playback", "Forward 5 seconds", Global, C::Fwd5s, &["Right", "Num9"]),
    b("vol_up", "Playback", "Volume up", Global, C::VolumeUp, &["Up", "Num8"]),
    b("vol_down", "Playback", "Volume down", Global, C::VolumeDown, &["Down", "Num2"]),
    b("tracks_back", "Playback", "Ten tracks back", Global, C::TenTracksBack, &["Num1"]),
    b("tracks_fwd", "Playback", "Ten tracks forward", Global, C::TenTracksForward, &["Num3"]),
    b("repeat", "Playback", "Repeat", Global, C::ToggleRepeat, &["R"]),
    b("shuffle", "Playback", "Shuffle", Global, C::ToggleShuffle, &["S"]),
    b("manual_advance", "Playback", "Manual playlist advance", Global, C::ToggleManualAdvance, &["Shift+R"]),
    b("stop_after", "Playback", "Stop after current track", Global, C::StopAfterCurrent, &["Ctrl+V"]),
    b("start_of_list", "Playback", "First track", Global, C::StartOfList, &["Ctrl+Z"]),
    b("end_of_list", "Playback", "Last track", Global, C::EndOfList, &["Ctrl+B"]),
    b("jump_time", "Playback", "Jump to time", Global, C::JumpToTime, &["Ctrl+J"]),
    b("time_mode", "Playback", "Elapsed or remaining time", Global, C::ToggleTimeMode, &["Ctrl+T"]),
    b("open_file", "Files", "Open file", Global, C::OpenFile, &["L", "Num0"]),
    b("open_folder", "Files", "Open folder", Global, C::OpenFolder, &["Shift+L", "Insert"]),
    b("open_url", "Files", "Open URL", Global, C::OpenUrl, &["Ctrl+L"]),
    b("jump_file", "Files", "Jump to file", Global, C::JumpToFile, &["J", "Num."]),
    b("file_info", "Files", "File info", Global, C::FileInfo, &["Alt+3"]),
    b("album_art", "Files", "Album art", Global, C::AlbumArt, &["Alt+A"]),
    b("history", "Files", "History", Global, C::History, &["Ctrl+H"]),
    b("bookmark_add", "Files", "Add bookmark", Global, C::AddBookmark, &["Alt+I"]),
    b("bookmark_remove", "Files", "Remove bookmark", Global, C::RemoveBookmark, &["Ctrl+Alt+I"]),
    b("prefs", "Windows", "Preferences", Global, C::Preferences, &["Ctrl+P"]),
    b("skins", "Windows", "Skin browser", Global, C::SkinBrowser, &["Alt+S"]),
    b("about", "Windows", "About", Global, C::About, &["F1"]),
    b("aot", "Windows", "Always on top", Global, C::ToggleAlwaysOnTop, &["Ctrl+A", "Ctrl+Alt+A"]),
    b("shade", "Windows", "Windowshade mode", Global, C::ToggleWindowshade, &["Ctrl+W"]),
    b("dsize", "Windows", "Double size", Global, C::ToggleDoubleSize, &["Ctrl+D"]),
    b("easymove", "Windows", "Easy move", Global, C::ToggleEasyMove, &["Ctrl+E"]),
    b("vis_toggle", "Windows", "Visualization on or off", Global, C::ToggleVis, &["Ctrl+Shift+K"]),
    b("vis_select", "Windows", "Select visualization", Global, C::SelectVisPlugin, &["Ctrl+K", "Alt+K"]),
    b("win_main", "Windows", "Main window", Global, C::ToggleMainWindow, &["Alt+W"]),
    b("win_playlist", "Windows", "Playlist window", Global, C::TogglePlaylistWindow, &["Alt+E"]),
    b("win_eq", "Windows", "Equalizer window", Global, C::ToggleEqWindow, &["Alt+G"]),
    b("win_library", "Windows", "Media Library window", Global, C::ToggleLibraryWindow, &["Alt+L"]),
    b("minimize", "Windows", "Minimize", Global, C::Minimize, &["Alt+M"]),
    b("pl_up", "Playlist window", "Move selection up", Playlist, C::MoveSelectionUp, &["Alt+Up"]),
    b("pl_down", "Playlist window", "Move selection down", Playlist, C::MoveSelectionDown, &["Alt+Down"]),
    b("pl_play", "Playlist window", "Play selected", Playlist, C::PlaySelected, &["Enter", "NumEnter"]),
    b("pl_clear", "Playlist window", "Clear playlist", Playlist, C::ClearPlaylist, &["Ctrl+Shift+Delete", "Ctrl+N"]),
    b("pl_crop", "Playlist window", "Crop to selected", Playlist, C::PlCropSelected, &["Ctrl+Delete"]),
    b("pl_missing", "Playlist window", "Remove missing files", Playlist, C::RemoveMissing, &["Alt+Delete"]),
    b("pl_remove", "Playlist window", "Remove selected", Playlist, C::PlRemoveSelected, &["Delete"]),
    b("pl_select_all", "Playlist window", "Select all", Playlist, C::SelectAll, &["Ctrl+A"]),
    b("pl_invert", "Playlist window", "Invert selection", Playlist, C::InvertSelection, &["Ctrl+I"]),
    b("pl_open", "Playlist window", "Open playlist", Playlist, C::PlaylistOpen, &["Ctrl+O"]),
    b("pl_save", "Playlist window", "Save playlist", Playlist, C::PlaylistSave, &["Ctrl+S"]),
    b("pl_edit", "Playlist window", "Edit entry", Playlist, C::EditPlaylistEntry, &["Ctrl+E"]),
    b("pl_shade", "Playlist window", "Playlist windowshade", Playlist, C::TogglePlShade, &["Ctrl+W"]),
    b("pl_randomize", "Playlist window", "Randomize list", Playlist, C::RandomizeList, &["Ctrl+Shift+R"]),
    b("pl_reverse", "Playlist window", "Reverse list", Playlist, C::ReverseList, &["Ctrl+R"]),
    b("pl_sort_title", "Playlist window", "Sort by title", Playlist, C::SortByTitle, &["Ctrl+Shift+1"]),
    b("pl_sort_file", "Playlist window", "Sort by file name", Playlist, C::SortByFileName, &["Ctrl+Shift+2"]),
    b("pl_sort_path", "Playlist window", "Sort by path", Playlist, C::SortByPath, &["Ctrl+Shift+3"]),
    b("pl_add_url", "Playlist window", "Add URL", Playlist, C::PlAddUrl, &["Ctrl+L"]),
    b("pl_add_folder", "Playlist window", "Add folder", Playlist, C::PlAddFolder, &["Shift+L", "Insert"]),
    b("pl_add_files", "Playlist window", "Add files", Playlist, C::PlAddFiles, &["L"]),
    b("pl_bookmark", "Playlist window", "Bookmark selected", Playlist, C::BookmarkSelected, &["Alt+I"]),
    b("eq_on", "Equalizer window", "Equalizer on or off", Equalizer, C::ToggleEq, &["N"]),
    b("eq_auto", "Equalizer window", "Equalizer auto-load", Equalizer, C::ToggleEqAutoload, &["A"]),
    b("eq_load", "Equalizer window", "Load equalizer preset", Equalizer, C::EqPresetLoad, &["Ctrl+Alt+S"]),
    b("eq_shade", "Equalizer window", "Equalizer windowshade", Equalizer, C::ToggleEqShade, &["Ctrl+W"]),
];

/// The keys of every shortcut now: as they came, or as the user set them.
#[derive(Debug, Clone)]
pub struct Keymap {
    /// The keys of each shortcut, in the order of [`BINDINGS`].
    chords: Vec<Vec<Chord>>,
}

fn defaults(binding: &Binding) -> Vec<Chord> {
    binding.keys.iter().filter_map(|k| Chord::parse(k)).collect()
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap { chords: BINDINGS.iter().map(defaults).collect() }
    }
}

impl Keymap {
    /// The keys, where `saved(id)` gives what the ini file says for a
    /// shortcut: its keys joined with `;`, empty for none. A shortcut the
    /// file does not mention has its own keys.
    pub fn load<'a>(saved: impl Fn(&str) -> Option<&'a str>) -> Keymap {
        let chords = BINDINGS
            .iter()
            .map(|b| match saved(b.id) {
                Some(text) => text.split(';').filter_map(Chord::parse).collect(),
                None => defaults(b),
            })
            .collect();
        Keymap { chords }
    }

    /// What a shortcut runs for key `code` with these modifiers held, among
    /// those of `scope`. A key pressed with Shift that has no shortcut of
    /// its own with Shift runs the one without it.
    pub fn find(&self, scope: Scope, code: K, ctrl: bool, shift: bool, alt: bool) -> Option<Command> {
        let with = |shift: bool| {
            BINDINGS.iter().zip(&self.chords).filter(|(b, _)| b.scope == scope).find_map(|(b, keys)| {
                keys.iter().any(|c| c.code == code && c.ctrl == ctrl && c.shift == shift && c.alt == alt).then_some(b.command)
            })
        };
        with(shift).or_else(|| if shift { with(false) } else { None })
    }

    /// The keys of shortcut `i`, as the list shows them.
    pub fn text(&self, i: usize) -> String {
        self.chords[i].iter().map(Chord::text).collect::<Vec<_>>().join(", ")
    }

    /// Shortcut `i` as the ini file keeps it.
    pub fn saved(&self, i: usize) -> String {
        self.chords[i].iter().map(Chord::text).collect::<Vec<_>>().join(";")
    }

    pub fn is_default(&self, i: usize) -> bool {
        self.chords[i] == defaults(&BINDINGS[i])
    }

    pub fn reset(&mut self, i: usize) {
        self.chords[i] = defaults(&BINDINGS[i]);
    }

    pub fn clear(&mut self, i: usize) {
        self.chords[i].clear();
    }

    /// Gives shortcut `i` the keys `chord`, in place of its own (`add`:
    /// besides them). Another shortcut of the same scope that had the keys
    /// loses them; its place in [`BINDINGS`] is returned.
    pub fn assign(&mut self, i: usize, chord: Chord, add: bool) -> Option<usize> {
        let scope = BINDINGS[i].scope;
        let mut lost = None;
        for (j, keys) in self.chords.iter_mut().enumerate() {
            if j != i && BINDINGS[j].scope == scope && keys.contains(&chord) {
                keys.retain(|c| *c != chord);
                lost = lost.or(Some(j));
            }
        }
        if !add {
            self.chords[i].clear();
        }
        if !self.chords[i].contains(&chord) {
            self.chords[i].push(chord);
        }
        lost
    }

    /// What a menu shows beside `command`: its first keys. None when the
    /// command has no shortcut that can be changed.
    pub fn accel(&self, command: Command) -> Option<String> {
        let i = BINDINGS.iter().position(|b| b.command == command)?;
        Some(self.chords[i].first().map(Chord::text).unwrap_or_default())
    }

    /// Writes the shortcuts the user changed into the menu items.
    pub fn relabel(&self, items: &mut [MenuItem]) {
        for item in items {
            if let Some(MenuAction::Cmd(c)) = &item.action {
                if let Some(text) = self.accel(*c) {
                    item.accel = text;
                }
            }
            self.relabel(&mut item.sub);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> Chord {
        Chord::parse(text).unwrap()
    }

    #[test]
    fn keys_are_written_with_their_modifiers_and_read_back() {
        for text in ["Z", "Ctrl+Shift+R", "Alt+Up", "Num4", "Num+", "Ctrl+Alt+Num.", "Ctrl+Shift+Delete", "F1", "`"] {
            assert_eq!(chord(text).text(), text);
        }
        assert_eq!(Chord::parse("ctrl+shift+r").map(|c| c.text()), Some("Ctrl+Shift+R".into()), "case does not matter");
        assert_eq!(Chord::parse("Ctrl+"), None);
        assert_eq!(Chord::parse("Hyper+Q"), None);
        assert!(Chord::new(K::ControlLeft, false, false, false).is_none(), "a modifier alone is no key");
    }

    #[test]
    fn every_shortcut_has_keys_that_read_and_no_two_of_a_scope_share_one() {
        let mut seen: Vec<(Scope, Chord, &str)> = Vec::new();
        for b in BINDINGS {
            for key in b.keys {
                let c = Chord::parse(key).unwrap_or_else(|| panic!("{} has the key {key}, which cannot be read", b.id));
                if let Some(other) = seen.iter().find(|s| s.0 == b.scope && s.1 == c) {
                    panic!("{} and {} both have {key}", b.id, other.2);
                }
                seen.push((b.scope, c, b.id));
            }
        }
        let mut ids: Vec<&str> = BINDINGS.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BINDINGS.len(), "the ids are all different");
    }

    #[test]
    fn a_key_runs_what_it_has_always_run() {
        let m = Keymap::default();
        let find = |scope, key, ctrl, shift, alt| m.find(scope, key, ctrl, shift, alt);
        assert_eq!(find(Global, K::KeyZ, false, false, false), Some(Command::Previous));
        assert_eq!(find(Global, K::Numpad5, false, false, false), Some(Command::Play));
        assert_eq!(find(Global, K::KeyV, false, true, false), Some(Command::StopFadeout));
        assert_eq!(find(Global, K::KeyV, false, false, false), Some(Command::Stop));
        assert_eq!(find(Global, K::KeyV, true, false, false), Some(Command::StopAfterCurrent));
        assert_eq!(find(Global, K::KeyA, true, false, false), Some(Command::ToggleAlwaysOnTop));
        assert_eq!(find(Playlist, K::KeyA, true, false, false), Some(Command::SelectAll), "the playlist's own first");
        assert_eq!(find(Playlist, K::KeyA, true, false, true), None, "Ctrl+Alt+A is the whole player's");
        assert_eq!(find(Global, K::KeyA, true, false, true), Some(Command::ToggleAlwaysOnTop));
        assert_eq!(find(Equalizer, K::KeyN, false, false, false), Some(Command::ToggleEq));
        assert_eq!(find(Global, K::KeyK, false, false, true), Some(Command::SelectVisPlugin));
        assert_eq!(find(Global, K::KeyX, false, true, false), Some(Command::Play), "Shift does not matter when no Shift shortcut has the key");
        assert_eq!(find(Global, K::KeyL, false, true, false), Some(Command::OpenFolder), "but one that has it wins");
        assert_eq!(find(Global, K::KeyQ, false, false, false), None);
    }

    #[test]
    fn a_changed_shortcut_is_kept_and_read_back_and_the_keys_it_took_are_taken_from_the_other() {
        let mut m = Keymap::default();
        let pause = BINDINGS.iter().position(|b| b.id == "pause").unwrap();
        let play = BINDINGS.iter().position(|b| b.id == "play").unwrap();
        assert!(m.is_default(pause));
        // Pause takes X, which Play had besides Num5.
        assert_eq!(m.assign(pause, chord("X"), false), Some(play));
        assert_eq!(m.text(pause), "X");
        assert_eq!(m.text(play), "Num5");
        assert_eq!(m.find(Global, K::KeyX, false, false, false), Some(Command::Pause));
        assert_eq!(m.find(Global, K::KeyC, false, false, false), None, "C is free now");
        assert!(!m.is_default(pause));
        // Added besides the others.
        m.assign(pause, chord("Ctrl+Space"), true);
        assert_eq!(m.text(pause), "X, Ctrl+Space");
        // Kept in the ini file as text, and read back.
        let saved = m.saved(pause);
        assert_eq!(saved, "X;Ctrl+Space");
        let back = Keymap::load(|id| (id == "pause").then_some(saved.as_str()));
        assert_eq!(back.text(pause), "X, Ctrl+Space");
        assert_eq!(back.text(play), "X, Num5", "the others are as they came: this test's file did not change Play");
        // Cleared, a shortcut has no keys; reset, its own.
        m.clear(pause);
        assert_eq!(m.text(pause), "");
        assert_eq!(Keymap::load(|id| (id == "pause").then_some("")).text(pause), "", "an empty line is no keys, not the defaults");
        m.reset(pause);
        assert!(m.is_default(pause));
    }

    #[test]
    fn a_shortcut_of_another_scope_may_share_keys() {
        let mut m = Keymap::default();
        let select_all = BINDINGS.iter().position(|b| b.id == "pl_select_all").unwrap();
        let aot = BINDINGS.iter().position(|b| b.id == "aot").unwrap();
        assert_eq!(m.assign(select_all, chord("Ctrl+A"), false), None, "Ctrl+A is the player's too, in the other scope");
        assert!(m.text(aot).contains("Ctrl+A"));
    }

    #[test]
    fn menus_show_the_keys_now_set() {
        let mut m = Keymap::default();
        let prefs = BINDINGS.iter().position(|b| b.id == "prefs").unwrap();
        let mut items = vec![MenuItem::cmd("Preferences", "Ctrl+P", Command::Preferences), MenuItem::cmd("Other", "Q", Command::Exit)];
        m.relabel(&mut items);
        assert_eq!(items[0].accel, "Ctrl+P");
        assert_eq!(items[1].accel, "Q", "a command with no changeable shortcut keeps its text");
        m.assign(prefs, chord("F5"), false);
        m.relabel(&mut items);
        assert_eq!(items[0].accel, "F5");
        m.clear(prefs);
        m.relabel(&mut items);
        assert_eq!(items[0].accel, "", "no keys: nothing shown");
    }
}
