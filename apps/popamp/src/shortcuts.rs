//! Title-bar shortcuts: what a right or middle click on the title bar (the
//! part that drags the window) of the main, equalizer or playlist window
//! does, whether the window is full-size or shaded. Chosen under
//! Preferences > Shortcuts, kept in the `title_shortcuts` setting as
//! `window:button:action` rules joined by `;`, for example
//! `eq:middle:pause;main:right:next`.

use popamp_core::Command;
use popamp_skin::layout::WindowKey;

/// The windows that have shortcuts.
pub const TARGETS: [(WindowKey, &str, &str); 3] = [
    (WindowKey::Main, "main", "Main window"),
    (WindowKey::Eq, "eq", "Equalizer window"),
    (WindowKey::Playlist, "playlist", "Playlist window"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Right,
    Middle,
}

/// The mouse buttons that have shortcuts.
pub const CLICKS: [(Click, &str, &str); 2] = [(Click::Right, "right", "Right Mouse"), (Click::Middle, "middle", "Middle Mouse")];

/// What a click can do.
pub const ACTIONS: [(Command, &str, &str); 6] = [
    (Command::Previous, "previous", "Previous track"),
    (Command::Next, "next", "Next track"),
    (Command::Pause, "pause", "Pause"),
    (Command::Stop, "stop", "Stop"),
    (Command::ToggleShuffle, "shuffle", "Shuffle on/off"),
    (Command::ToggleRepeat, "repeat", "Repeat on/off"),
];

/// One rule: a click on a window's title bar runs a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub window: WindowKey,
    pub click: Click,
    pub command: Command,
}

/// The rules in `spec`; what does not read as a rule is left out.
pub fn parse(spec: &str) -> Vec<Rule> {
    spec.split(';')
        .filter_map(|rule| {
            let mut parts = rule.split(':').map(str::trim);
            let (w, c, a) = (parts.next()?, parts.next()?, parts.next()?);
            if parts.next().is_some() {
                return None;
            }
            Some(Rule {
                window: TARGETS.iter().find(|t| t.1 == w)?.0,
                click: CLICKS.iter().find(|t| t.1 == c)?.0,
                command: ACTIONS.iter().find(|t| t.1 == a)?.0,
            })
        })
        .collect()
}

/// The setting that holds `rules`.
pub fn format(rules: &[Rule]) -> String {
    let name = |w: WindowKey, c: Click, a: Command| {
        let w = TARGETS.iter().find(|t| t.0 == w).map_or("", |t| t.1);
        let c = CLICKS.iter().find(|t| t.0 == c).map_or("", |t| t.1);
        let a = ACTIONS.iter().find(|t| t.0 == a).map_or("", |t| t.1);
        format!("{w}:{c}:{a}")
    };
    rules.iter().map(|r| name(r.window, r.click, r.command)).collect::<Vec<_>>().join(";")
}

/// What a click on the title bar of window `window` runs under `spec`;
/// of two rules for the same click, the first counts.
pub fn find(spec: &str, window: WindowKey, click: Click) -> Option<Command> {
    parse(spec).into_iter().find(|r| r.window == window && r.click == click).map(|r| r.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_is_found_by_window_and_button() {
        let spec = "eq:middle:pause;main:right:next";
        assert_eq!(find(spec, WindowKey::Eq, Click::Middle), Some(Command::Pause));
        assert_eq!(find(spec, WindowKey::Main, Click::Right), Some(Command::Next));
        assert_eq!(find(spec, WindowKey::Eq, Click::Right), None);
        assert_eq!(find(spec, WindowKey::Playlist, Click::Middle), None);
        assert_eq!(find("", WindowKey::Main, Click::Right), None);
    }

    #[test]
    fn the_first_of_two_rules_for_a_click_counts() {
        assert_eq!(find("main:right:stop;main:right:next", WindowKey::Main, Click::Right), Some(Command::Stop));
    }

    #[test]
    fn rules_survive_being_written_and_read() {
        let rules: Vec<Rule> = ACTIONS
            .iter()
            .map(|a| Rule { window: WindowKey::Playlist, click: Click::Middle, command: a.0 })
            .chain([Rule { window: WindowKey::Main, click: Click::Right, command: Command::Next }])
            .collect();
        assert_eq!(parse(&format(&rules)), rules);
    }

    #[test]
    fn what_is_not_a_rule_is_left_out() {
        let spec = "nonsense; main:right:fly ; main:left:next; main:right:next:extra; Eq:middle:pause; playlist : middle : repeat";
        assert_eq!(parse(spec), [Rule { window: WindowKey::Playlist, click: Click::Middle, command: Command::ToggleRepeat }]);
    }
}
