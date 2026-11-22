//! The Folder and Group Editor of the Media Library's folder tree: a list
//! of the groups and folders to add to, rename, take away, reorder (with
//! the buttons or by dragging) and move in and out of groups.

use crate::foldertree::{clean_name, normalize, Item};
use crate::model::Model;
use crate::panels::{button, Act};
use crate::widgets::{Event, Form, Rect, Widget, BUTTON, ROW_H};

const WIDTH: i32 = 570;
const LIST_ROWS: i32 = 14;
const SIDE_W: i32 = 120;

pub struct FolderEditor {
    pub(crate) form: Form<Act>,
    list: usize,
    name: usize,
    path: usize,
    /// The box that shows or hides the Bookmarks folder.
    bookmarks: usize,
    /// The box that shows or hides the History folder.
    history: usize,
    /// The groups and folders being edited; they replace the tree on OK.
    pub(crate) items: Vec<Item>,
    /// The row whose name and path the two fields hold.
    shown: Option<usize>,
    /// The row the mouse holds, to move it.
    held: Option<usize>,
}

/// The number of rows a group takes with the folders in it; a folder is one.
fn unit_len(items: &[Item], k: usize) -> usize {
    match items.get(k) {
        Some(Item::Group { .. }) => 1 + items[k + 1..].iter().take_while(|i| matches!(i, Item::Folder { child: true, .. })).count(),
        Some(_) => 1,
        None => 0,
    }
}

/// Whether row `k` is a folder in a group.
fn is_child(items: &[Item], k: usize) -> bool {
    matches!(items.get(k), Some(Item::Folder { child: true, .. }))
}

/// Moves the group or folder at `k` so that it lands by row `t` (a row of
/// the list as it is now) and returns where it is after. A folder dropped
/// below a group or one of its folders is in the group; a group goes
/// between whole groups.
pub fn move_to(items: &mut Vec<Item>, k: usize, t: usize) -> usize {
    let n = unit_len(items, k);
    if n == 0 || (t >= k && t < k + n) {
        return k;
    }
    let mut block: Vec<Item> = items.drain(k..k + n).collect();
    let mut pos = if t < k { t } else { t + 1 - n };
    if matches!(block[0], Item::Group { .. }) {
        if t < k {
            while pos > 0 && is_child(items, pos) {
                pos -= 1;
            }
        } else {
            while is_child(items, pos) {
                pos += 1;
            }
        }
    }
    if let Item::Folder { child, .. } = &mut block[0] {
        *child = pos > 0
            && match &items[pos - 1] {
                Item::Group { .. } => true,
                Item::Folder { child, .. } => *child,
            };
    }
    let at = pos.min(items.len());
    for (i, it) in block.into_iter().enumerate() {
        items.insert(at + i, it);
    }
    normalize(items);
    at
}

/// One step up: a folder passes the row above, a group the group or folder above it.
pub fn step_up(items: &mut Vec<Item>, k: usize) -> usize {
    if k == 0 {
        return k;
    }
    let mut to = k - 1;
    if matches!(items[k], Item::Group { .. }) {
        while to > 0 && is_child(items, to) {
            to -= 1;
        }
    }
    move_to(items, k, to)
}

/// One step down.
pub fn step_down(items: &mut Vec<Item>, k: usize) -> usize {
    let next = k + unit_len(items, k);
    if next >= items.len() {
        return k;
    }
    let to = if matches!(items[k], Item::Group { .. }) { next + unit_len(items, next) - 1 } else { next };
    move_to(items, k, to)
}

/// Puts the folder at `k` in the group above it, when there is one.
pub fn indent(items: &mut [Item], k: usize) {
    let ok = k > 0 && matches!(items[k - 1], Item::Group { .. }) || is_child(items, k.wrapping_sub(1));
    if let (true, Some(Item::Folder { child, .. })) = (ok, items.get_mut(k)) {
        *child = true;
    }
}

/// Takes the folder at `k` out of its group, to just after the group's folders.
pub fn outdent(items: &mut Vec<Item>, k: usize) -> usize {
    if !is_child(items, k) {
        return k;
    }
    let mut group = k;
    while is_child(items, group) {
        group -= 1;
    }
    let end = group + unit_len(items, group);
    let Item::Folder { path, name, .. } = items.remove(k) else { return k };
    let at = end - 1;
    items.insert(at, Item::Folder { path, name, child: false });
    normalize(items);
    at
}

impl FolderEditor {
    pub fn new(model: &Model) -> FolderEditor {
        let list_h = 4 + LIST_ROWS * ROW_H;
        let list_w = WIDTH - 30 - SIDE_W;
        let mut form = Form::new(WIDTH, 0);
        let list = form.add(Rect::new(10, 10, list_w, list_h), Widget::List { rows: vec![], selected: None, scroll: 0 });
        let side = |n: i32| Rect::new(WIDTH - 10 - SIDE_W, 10 + n * 30, SIDE_W, BUTTON.1);
        for (n, (label, act)) in [
            ("Add folder", Act::FolderAdd),
            ("Add group", Act::GroupAdd),
            ("Remove", Act::FolderRemove),
            ("Move up", Act::FolderUp),
            ("Move down", Act::FolderDown),
            ("Into group", Act::FolderIn),
            ("Out of group", Act::FolderOut),
        ]
        .into_iter()
        .enumerate()
        {
            form.add(side(n as i32), Widget::Button { label: label.into(), action: act, default: false });
        }
        let y = 10 + list_h + 12;
        form.add(Rect::new(14, y, 60, 24), Widget::Label { text: "Name:".into(), dim: false });
        let name = form.add(Rect::new(80, y, WIDTH - 90, 24), Widget::Input { text: String::new(), cursor: 0 });
        form.add(Rect::new(14, y + 32, 60, 24), Widget::Label { text: "Path:".into(), dim: false });
        let path = form.add(Rect::new(80, y + 32, WIDTH - 90, 24), Widget::Input { text: String::new(), cursor: 0 });
        let by = y + 70;
        let shown = model.library.tree.items.iter().any(Item::is_bookmarks);
        let bookmarks = form.add(
            Rect::new(12, by, 130, 24),
            Widget::Check { label: "Show Bookmarks folder".into(), checked: shown, action: Act::ShowBookmarks },
        );
        let history = form.add(
            Rect::new(150, by, 150, 24),
            Widget::Check { label: "Show History folder".into(), checked: model.library.tree.items.iter().any(Item::is_history), action: Act::ShowHistory },
        );
        button(&mut form, WIDTH - 3 * BUTTON.0 - 22, by, "Apply", Act::Apply, false);
        button(&mut form, WIDTH - 2 * BUTTON.0 - 16, by, "OK", Act::Ok, true);
        button(&mut form, WIDTH - BUTTON.0 - 10, by, "Cancel", Act::Close, false);
        form.h = by + BUTTON.1 + 10;
        let mut editor = FolderEditor { form, list, name, path, bookmarks, history, items: model.library.tree.items.clone(), shown: None, held: None };
        let first = editor.items.iter().position(Item::is_folder).or(if editor.items.is_empty() { None } else { Some(0) });
        editor.refresh(first);
        editor.load(first);
        editor
    }

    /// What a row says.
    fn row_text(items: &[Item], k: usize) -> String {
        match &items[k] {
            Item::Group { name, .. } => format!("[{name}]"),
            Item::Folder { path, child, .. } => {
                let indent = if *child { "     " } else { "" };
                let place = if crate::foldertree::is_bookmarks(path) {
                    "(your bookmarks)"
                } else if crate::foldertree::is_history(path) {
                    "(what you played lately)"
                } else {
                    path
                };
                format!("{indent}{}   {place}", items[k].label())
            }
        }
    }

    /// Writes the list again, with row `select` picked.
    fn refresh(&mut self, select: Option<usize>) {
        let shown = self.items.iter().any(Item::is_bookmarks);
        self.form.set_checked(self.bookmarks, shown);
        self.form.set_checked(self.history, self.items.iter().any(Item::is_history));
        let rows = (0..self.items.len()).map(|k| Self::row_text(&self.items, k)).collect();
        self.form.set_rows(self.list, rows, select);
    }

    /// Puts the name and path of row `k` in the two fields.
    fn load(&mut self, k: Option<usize>) {
        self.shown = k.filter(|&k| k < self.items.len());
        let (name, path) = match self.shown.map(|k| &self.items[k]) {
            Some(it @ Item::Group { .. }) => (it.label(), String::new()),
            Some(it @ Item::Folder { .. }) if it.is_bookmarks() => (it.label(), String::from("(the players bookmarks)")),
            Some(it @ Item::Folder { .. }) if it.is_history() => (it.label(), String::from("(the players history)")),
            Some(it @ Item::Folder { path, .. }) => (it.label(), path.clone()),
            None => (String::new(), String::new()),
        };
        self.form.set_text(self.name, &name);
        self.form.set_text(self.path, &path);
    }

    /// Takes what the fields say into the row they were showing.
    fn commit(&mut self) {
        let Some(k) = self.shown.filter(|&k| k < self.items.len()) else { return };
        let (name, path) = (clean_name(self.form.text(self.name)), self.form.text(self.path).trim().to_string());
        match &mut self.items[k] {
            Item::Group { name: old, .. } => {
                if !name.is_empty() {
                    *old = name;
                }
            }
            Item::Folder { path: old_path, name: old, .. } => {
                // Bookmarks and History have no path to change.
                if !crate::foldertree::is_virtual(old_path) {
                    *old_path = path;
                }
                let own = crate::foldertree::path_label(old_path);
                *old = if name == own { String::new() } else { name };
            }
        }
    }

    fn selected(&self) -> Option<usize> {
        self.form.selected(self.list)
    }

    /// Shows the list after a change that leaves `k` picked.
    fn picked(&mut self, k: usize) {
        self.refresh(Some(k));
        self.load(Some(k));
    }

    pub fn handle(&mut self, ev: Event<Act>, model: &mut Model) -> bool {
        match ev {
            Event::Selected => {
                self.commit();
                self.refresh_keep();
                let k = self.selected();
                self.load(k);
            }
            Event::Edited => {
                self.commit();
                self.refresh_keep();
            }
            Event::Toggled(Act::ShowBookmarks, on) => {
                self.commit();
                if on && !self.items.iter().any(Item::is_bookmarks) {
                    let at = self.items.len();
                    self.items.push(Item::Folder { path: crate::foldertree::BOOKMARKS.into(), name: "Bookmarks".into(), child: false });
                    self.picked(at);
                } else if !on {
                    self.items.retain(|i| !i.is_bookmarks());
                    normalize(&mut self.items);
                    let k = self.selected().unwrap_or(0).min(self.items.len().saturating_sub(1));
                    self.refresh((!self.items.is_empty()).then_some(k));
                    self.load((!self.items.is_empty()).then_some(k));
                }
            }
            Event::Toggled(Act::ShowHistory, on) => {
                self.commit();
                if on && !self.items.iter().any(Item::is_history) {
                    let at = self.items.len();
                    self.items.push(Item::Folder { path: crate::foldertree::HISTORY.into(), name: "History".into(), child: false });
                    self.picked(at);
                } else if !on {
                    self.items.retain(|i| !i.is_history());
                    normalize(&mut self.items);
                    let k = self.selected().unwrap_or(0).min(self.items.len().saturating_sub(1));
                    self.refresh((!self.items.is_empty()).then_some(k));
                    self.load((!self.items.is_empty()).then_some(k));
                }
            }
            Event::Clicked(Act::FolderAdd) => {
                self.commit();
                // Under the row picked: in its group when it is a group or in one.
                let (at, child) = match self.selected() {
                    Some(k) => (k + 1, matches!(self.items[k], Item::Group { .. }) || is_child(&self.items, k)),
                    None => (self.items.len(), false),
                };
                self.items.insert(at, Item::Folder { path: String::new(), name: "New folder".into(), child });
                normalize(&mut self.items);
                self.picked(at);
            }
            Event::Clicked(Act::GroupAdd) => {
                self.commit();
                let at = self.selected().map_or(self.items.len(), |k| k + unit_len(&self.items, k));
                self.items.insert(at, Item::Group { name: "New group".into(), open: true });
                normalize(&mut self.items);
                self.picked(at);
            }
            Event::Clicked(Act::FolderRemove) => {
                self.commit();
                if let Some(k) = self.selected() {
                    self.items.remove(k);
                    normalize(&mut self.items);
                    let next = k.min(self.items.len().saturating_sub(1));
                    self.refresh((!self.items.is_empty()).then_some(next));
                    self.load((!self.items.is_empty()).then_some(next));
                }
            }
            Event::Clicked(Act::FolderUp) => self.change(step_up),
            Event::Clicked(Act::FolderDown) => self.change(step_down),
            Event::Clicked(Act::FolderIn) => {
                self.change(|items, k| {
                    indent(items, k);
                    k
                });
            }
            Event::Clicked(Act::FolderOut) => self.change(outdent),
            Event::Clicked(Act::Apply) => {
                // The tree takes the edits; the editor stays open to go on.
                self.commit();
                let items = self.items.iter().filter(|i| !matches!(i, Item::Folder { path, .. } if path.is_empty())).cloned().collect();
                model.library_set_tree(items);
            }
            Event::Clicked(Act::Ok) => {
                self.commit();
                let items = std::mem::take(&mut self.items).into_iter().filter(|i| !matches!(i, Item::Folder { path, .. } if path.is_empty())).collect();
                model.library_set_tree(items);
                return false;
            }
            Event::Clicked(_) | Event::Cancel => return false,
            _ => {}
        }
        true
    }

    /// Writes the rows again, keeping the pick.
    fn refresh_keep(&mut self) {
        let k = self.selected();
        self.refresh(k);
    }

    /// Changes the list with `f(items, picked row) -> the row it is at after`.
    fn change(&mut self, f: impl FnOnce(&mut Vec<Item>, usize) -> usize) {
        self.commit();
        if let Some(k) = self.selected() {
            let to = f(&mut self.items, k);
            self.picked(to);
        }
    }

    // --- dragging ---------------------------------------------------------------

    /// The mouse went down at `(x, y)`: a row of the list is held.
    pub fn mouse_press(&mut self, x: i32, y: i32) {
        self.held = self.form.list_row_at(self.list, x, y);
    }

    /// The mouse moved with the button down: the row held follows it.
    pub fn mouse_drag(&mut self, _x: i32, y: i32) -> bool {
        let Some(k) = self.held else { return false };
        let Some(t) = self.form.list_row_near(self.list, y) else { return false };
        if t == k {
            return false;
        }
        let to = move_to(&mut self.items, k, t);
        self.held = Some(to);
        self.shown = Some(to);
        self.refresh(Some(to));
        true
    }

    pub fn mouse_release(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, child: bool) -> Item {
        Item::Folder { path: format!("/m/{name}"), name: String::new(), child }
    }

    fn group(name: &str) -> Item {
        Item::Group { name: name.into(), open: true }
    }

    fn describe(items: &[Item]) -> String {
        items
            .iter()
            .map(|i| match i {
                Item::Group { name, .. } => format!("[{name}]"),
                Item::Folder { path, child, .. } => format!("{}{}", if *child { ">" } else { "" }, path.trim_start_matches("/m/")),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn sample() -> Vec<Item> {
        vec![folder("a", false), group("G"), folder("b", true), folder("c", true), folder("d", false)]
    }

    #[test]
    fn a_folder_dropped_below_a_group_or_its_folders_is_in_it() {
        let mut v = sample();
        assert_eq!(move_to(&mut v, 0, 1), 1, "a goes below the group header: first in G");
        assert_eq!(describe(&v), "[G] >a >b >c d");
        let mut v = sample();
        move_to(&mut v, 4, 2);
        assert_eq!(describe(&v), "a [G] >d >b >c", "d dropped on b: in G, before it");
        let mut v = sample();
        move_to(&mut v, 2, 0);
        assert_eq!(describe(&v), "b a [G] >c d", "b dropped on top: out of the group");
    }

    #[test]
    fn a_group_moves_with_its_folders_and_goes_between_whole_groups() {
        let mut v = sample();
        move_to(&mut v, 1, 4);
        assert_eq!(describe(&v), "a d [G] >b >c");
        let mut v = vec![group("A"), folder("x", true), group("B"), folder("y", true), folder("z", true)];
        let at = move_to(&mut v, 2, 1);
        assert_eq!(describe(&v), "[B] >y >z [A] >x", "B above the folder of A goes above A");
        assert_eq!(at, 0);
    }

    #[test]
    fn the_buttons_step_a_row_and_move_it_in_and_out_of_a_group() {
        let mut v = sample();
        assert_eq!(step_down(&mut v, 0), 1);
        assert_eq!(describe(&v), "[G] >a >b >c d", "a steps below the group header: into G");
        assert_eq!(outdent(&mut v, 1), 3);
        assert_eq!(describe(&v), "[G] >b >c a d", "out of G, after its other folders");
        indent(&mut v, 3);
        assert_eq!(describe(&v), "[G] >b >c >a d");
        indent(&mut v, 4);
        assert_eq!(describe(&v), "[G] >b >c >a >d", "d under a folder of G goes in too");
        assert_eq!(step_up(&mut v, 4), 3);
        let mut v = sample();
        assert_eq!(step_down(&mut v, 1), 2, "G with its folders passes d");
        assert_eq!(describe(&v), "a d [G] >b >c");
        assert_eq!(step_up(&mut v, 2), 1);
        assert_eq!(describe(&v), "a [G] >b >c d");
    }

    #[test]
    fn only_a_folder_under_a_group_can_be_in_it() {
        let mut v = vec![folder("a", true), folder("b", false), folder("c", true)];
        normalize(&mut v);
        assert_eq!(describe(&v), "a b c");
        indent(&mut v, 1);
        assert_eq!(describe(&v), "a b c", "nothing above to go into");
    }

    #[test]
    fn the_editor_edits_a_copy_that_replaces_the_tree_on_ok() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        // The first folder is picked, with its name and path in the fields.
        assert_eq!(e.shown, Some(0));
        assert_eq!(e.form.text(e.path), "/m/a");
        e.form.set_text(e.name, "Alpha");
        assert!(e.handle(Event::Edited, &mut m));
        assert_eq!(e.items[0].label(), "Alpha");
        assert_eq!(m.library.tree.items[0].label(), "a", "not yet");
        assert!(e.handle(Event::Clicked(Act::GroupAdd), &mut m));
        assert_eq!(e.items.len(), 6);
        assert!(!e.handle(Event::Clicked(Act::Ok), &mut m), "OK closes");
        assert_eq!(m.library.tree.items[0].label(), "Alpha");
        assert_eq!(m.library.tree.items.len(), 6);
        assert_eq!(m.config.folder_tree().map(|l| l.len()), Some(6), "kept in the ini file");
    }

    #[test]
    fn the_show_bookmarks_box_puts_the_bookmarks_folder_in_the_list_or_takes_it_out() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        let n = e.items.len();
        assert!(!e.items.iter().any(Item::is_bookmarks), "not there at first");
        assert!(e.handle(Event::Toggled(Act::ShowBookmarks, true), &mut m));
        assert_eq!(e.items.len(), n + 1);
        assert!(e.items.last().is_some_and(Item::is_bookmarks), "at the end, like any folder, to be moved as the others");
        assert_eq!(e.items.last().map(Item::label), Some("Bookmarks".to_string()));
        // Moved up like a folder.
        let at = e.items.len() - 1;
        e.items.swap(at - 1, at);
        // OK: the library's left list has it.
        assert!(!e.handle(Event::Clicked(Act::Ok), &mut m));
        assert!(m.library.tree.items.iter().any(Item::is_bookmarks));
        assert_eq!(m.config.folder_tree().map(|l| l.iter().any(|x| x.ends_with("|popamp:bookmarks"))), Some(true), "kept in the ini file");
        // Open again: the box is ticked; take it away and OK.
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        assert!(matches!(e.form.widget(e.bookmarks), Widget::Check { checked: true, .. }));
        assert!(e.handle(Event::Toggled(Act::ShowBookmarks, false), &mut m));
        assert!(!e.items.iter().any(Item::is_bookmarks));
        assert!(!e.handle(Event::Clicked(Act::Ok), &mut m));
        assert!(!m.library.tree.items.iter().any(Item::is_bookmarks), "hidden again");
    }

    #[test]
    fn the_show_history_box_puts_the_history_folder_in_the_list_and_it_lists_the_plays() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        assert!(e.handle(Event::Toggled(Act::ShowHistory, true), &mut m));
        assert!(e.items.last().is_some_and(Item::is_history));
        assert!(!e.handle(Event::Clicked(Act::Ok), &mut m));
        assert!(m.library.tree.items.iter().any(Item::is_history));
        assert_eq!(m.config.folder_tree().map(|l| l.iter().any(|x| x.ends_with("|popamp:history"))), Some(true));
        m.library.tree.set_history(vec![("/m/b.mp3".into(), "B".into()), ("/m/a.mp3".into(), "A".into())]);
        let last = m.library.tree.items.len() - 1;
        m.library.tree.set_items(vec![m.library.tree.items[last].clone()]);
        m.library.tree.set_history(vec![("/m/b.mp3".into(), "B".into()), ("/m/a.mp3".into(), "A".into())]);
        assert_eq!(m.library.tree.jump_rows().len(), 2, "the plays are in the folder");
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        assert!(matches!(e.form.widget(e.history), Widget::Check { checked: true, .. }));
        assert!(e.handle(Event::Toggled(Act::ShowHistory, false), &mut m));
        assert!(!e.items.iter().any(Item::is_history));
    }

    #[test]
    fn apply_gives_the_tree_the_edits_and_keeps_the_editor_open() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        e.form.set_text(e.name, "Alpha");
        assert!(e.handle(Event::Edited, &mut m));
        assert!(e.handle(Event::Clicked(Act::Apply), &mut m), "still open");
        assert_eq!(m.library.tree.items[0].label(), "Alpha");
        assert_eq!(e.items.len(), 5, "the editor still has its rows");
    }

    #[test]
    fn cancel_forgets_the_edits_and_a_folder_without_a_path_is_dropped() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        e.handle(Event::Clicked(Act::FolderRemove), &mut m);
        assert!(!e.handle(Event::Cancel, &mut m));
        assert_eq!(m.library.tree.items.len(), 5);
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        e.handle(Event::Clicked(Act::FolderAdd), &mut m);
        assert_eq!(e.items.len(), 6);
        assert!(!e.handle(Event::Clicked(Act::Ok), &mut m));
        assert_eq!(m.library.tree.items.len(), 5, "the new folder has no path yet");
    }

    #[test]
    fn dragging_a_row_moves_it_to_the_row_under_the_mouse() {
        use crate::panels::{Panel, PanelKind};
        let mut m = Model::for_tests();
        m.library.tree.set_items(sample());
        let mut p = Panel::open(PanelKind::FolderEditor, &m);
        let Panel::Folders(e) = &mut p else { panic!() };
        let r = e.form.rect(e.list);
        let y_of = |row: i32| r.y + 2 + row * ROW_H + ROW_H / 2;
        e.mouse_press(r.x + 5, y_of(4));
        assert_eq!(e.held, Some(4));
        assert!(e.mouse_drag(r.x + 5, y_of(2)));
        assert_eq!(describe(&e.items), "a [G] >d >b >c", "d is in the group where it was dropped");
        assert!(!e.mouse_drag(r.x + 5, y_of(2)), "no move, no redraw");
        e.mouse_release();
        assert!(!e.mouse_drag(r.x + 5, y_of(0)), "let go: nothing is held");
    }
}
