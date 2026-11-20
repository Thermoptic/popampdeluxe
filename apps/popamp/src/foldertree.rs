//! The media library as a folder tree: the other kind of Media Library,
//! chosen in Preferences. It reads no tags, only names. The left list holds
//! the folders the user added, the right one shows what is in the folder
//! picked there, as Windows Explorer does: folders first, then the audio
//! files, named as their files are without the extension. A double click on
//! a folder goes into it, `..` goes back up, and the headers sort by name or
//! by date. A double click on a file plays it or adds it to the playlist,
//! as in the other library.
//!
//! The window is the library's: its parts, buttons and drawing helpers come
//! from `library.rs`, and `LibraryPane` hands every call to this when the
//! folder tree is on.

use crate::canvas::Canvas;
use crate::font::Font;
use crate::library::{CARRY_SLACK, Carry, List, Parts, Pen, Place, Reaction, Width, lay, rect};
use crate::widgets::Key;
use popamp_core::Command;
use popamp_core::text::natural_cmp;
use popamp_skin::bitmap::mix;
use popamp_skin::{PlColors, Rect};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The buttons under the lists. Rescan reads the folder on screen again;
/// Add folder, at the right of it, adds a folder to the left list.
pub const BUTTONS: [(&str, Command); 4] = [
    ("Play", Command::LibraryPlay),
    ("Enqueue", Command::LibraryEnqueue),
    ("Rescan", Command::LibraryRescan),
    ("Add folder", Command::LibraryAddFolder),
];

/// What the right list is ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Date,
}

/// A row of the right list.
#[derive(Debug, Clone)]
pub struct Entry {
    /// What the row says: the name of the folder, or of the file without its extension.
    pub name: String,
    pub path: String,
    pub dir: bool,
    /// The row `..`, which goes up to the folder above.
    pub up: bool,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub struct FolderTree {
    /// The folders of the left list.
    pub roots: Vec<String>,
    root: Option<usize>,
    /// The folder the right list shows: the root or one inside it.
    dir: Option<PathBuf>,
    entries: Vec<Entry>,
    sort: Option<(SortBy, bool)>,
    selected: Vec<bool>,
    cursor: usize,
    anchor: usize,
    /// First visible row of the roots and of the entries.
    scroll: [usize; 2],
    focus: Option<List>,
    pressed: Option<usize>,
    dragging: Option<(List, i32)>,
    carry: Option<Carry>,
    /// The folder on screen could not be read.
    unreadable: bool,
}

/// Whether `path` is a file Popamp plays.
fn is_audio(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| popamp_core::audio_extensions().any(|x| x.eq_ignore_ascii_case(e)))
}

/// The name of the last part of `path`.
fn label(path: &str) -> String {
    Path::new(path).file_name().map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

/// A file's time as Explorer writes it: `2026-11-18 20:41`, in local time.
fn date_text(at: SystemTime) -> String {
    jiff::Timestamp::try_from(at)
        .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn slot(list: List) -> usize {
    usize::from(list == List::Tracks)
}

impl FolderTree {
    /// The folders kept in `file`, one to a line.
    pub fn load(file: &Path) -> FolderTree {
        let mut tree = FolderTree::default();
        for line in std::fs::read_to_string(file).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !tree.roots.iter().any(|r| r == line) {
                tree.roots.push(line.to_string());
            }
        }
        if !tree.roots.is_empty() {
            tree.pick_root(0);
        }
        tree
    }

    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file, self.roots.join("\n") + "\n")
    }

    /// Adds `folder` to the left list and shows it. False when it is there already.
    pub fn add_root(&mut self, folder: &str) -> bool {
        if let Some(i) = self.roots.iter().position(|r| r == folder) {
            self.pick_root(i);
            return false;
        }
        self.roots.push(folder.to_string());
        self.pick_root(self.roots.len() - 1);
        true
    }

    /// Takes the picked folder out of the left list; its files stay.
    pub fn remove_root(&mut self) {
        let Some(i) = self.root else { return };
        self.roots.remove(i);
        self.scroll = [0, 0];
        match self.roots.len() {
            0 => (self.root, self.dir) = (None, None),
            n => return self.pick_root(i.min(n - 1)),
        }
        self.reload();
    }

    /// The folder picked in the left list.
    pub fn folder(&self) -> Option<&str> {
        self.root.and_then(|i| self.roots.get(i)).map(String::as_str)
    }

    fn pick_root(&mut self, i: usize) {
        self.root = Some(i);
        self.dir = self.roots.get(i).map(PathBuf::from);
        self.scroll[1] = 0;
        self.reload();
    }

    /// The folder on screen: the picked folder, or one inside it.
    pub fn current(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Reads the folder on screen again.
    pub fn reload(&mut self) {
        self.entries.clear();
        self.unreadable = false;
        if let Some(dir) = self.dir.clone() {
            match std::fs::read_dir(&dir) {
                Ok(read) => {
                    for e in read.flatten() {
                        let name = e.file_name().to_string_lossy().into_owned();
                        let Ok(meta) = e.metadata() else { continue };
                        if name.starts_with('.') {
                            continue;
                        }
                        let path = e.path();
                        let name = if meta.is_dir() {
                            name
                        } else if is_audio(&path) {
                            path.file_stem().map_or(name, |s| s.to_string_lossy().into_owned())
                        } else if popamp_core::plformats::is_playlist(&name) {
                            // A playlist keeps its extension, to tell it from a song.
                            name
                        } else {
                            continue;
                        };
                        let path = path.to_string_lossy().into_owned();
                        self.entries.push(Entry { name, path, dir: meta.is_dir(), up: false, modified: meta.modified().ok() });
                    }
                }
                Err(e) => {
                    log::warn!("folder tree: cannot read {}: {e}", dir.display());
                    self.unreadable = true;
                }
            }
        }
        self.order();
        let above = self.dir.as_deref().filter(|d| self.folder().map(Path::new) != Some(*d)).and_then(Path::parent);
        if let Some(parent) = above {
            let up = Entry { name: "..".into(), path: parent.to_string_lossy().into_owned(), dir: true, up: true, modified: None };
            self.entries.insert(0, up);
        }
        self.selected = vec![false; self.entries.len()];
        (self.cursor, self.anchor) = (0, 0);
        self.scroll[1] = 0;
    }

    /// Puts folders first, then the files, each in the order asked for.
    fn order(&mut self) {
        let (by, reverse) = self.sort.unwrap_or((SortBy::Name, false));
        self.entries.sort_by(|a, b| {
            b.dir.cmp(&a.dir).then_with(|| {
                let o = match by {
                    SortBy::Name => natural_cmp(&a.name, &b.name),
                    SortBy::Date => a.modified.cmp(&b.modified).then_with(|| natural_cmp(&a.name, &b.name)),
                };
                if reverse { o.reverse() } else { o }
            })
        });
    }

    /// Sorts by `by`; the same column again turns the order round. Names
    /// start at A, dates with the newest.
    fn sort_by(&mut self, by: SortBy) {
        self.sort = Some(match self.sort {
            Some((b, reverse)) if b == by => (by, !reverse),
            _ => (by, by == SortBy::Date),
        });
        self.reload();
    }

    /// Goes into the folder `path`.
    fn enter(&mut self, path: &str) {
        self.dir = Some(PathBuf::from(path));
        self.reload();
    }

    /// Goes up to the folder above, unless this is the picked one.
    fn up(&mut self) {
        if let Some(up) = self.entries.first().filter(|e| e.up).map(|e| e.path.clone()) {
            self.enter(&up);
        }
    }

    /// The paths of the picked rows: files and folders, in the order of the
    /// list; all the files when none is picked. A folder plays all it holds.
    pub fn picked(&self) -> Vec<String> {
        let some = self.selected.contains(&true);
        let rows = self.entries.iter().zip(&self.selected).filter(|(e, &on)| !e.up && if some { on } else { !e.dir });
        rows.map(|(e, _)| e.path.clone()).collect()
    }

    /// What the window's small display says.
    pub fn status(&self) -> String {
        if self.root.is_none() {
            return String::new();
        }
        let n = self.entries.iter().filter(|e| !e.up).count();
        if n == 1 { "1 item".into() } else { format!("{n} items") }
    }

    // --- scrolling ------------------------------------------------------------

    fn len(&self, list: List) -> usize {
        if list == List::Views { self.roots.len() } else { self.entries.len() }
    }

    fn window(&self, p: &Parts, list: List) -> (usize, usize) {
        let page = p.rows(list).map_or(1, |r| p.page(r));
        (self.scroll[slot(list)].min(self.len(list).saturating_sub(page)), page)
    }

    /// Where the thumb of `list`'s bar is (as in the other library).
    fn thumb(&self, p: &Parts, list: List) -> Option<Rect> {
        let bar = p.bar(list)?;
        let (scroll, page) = self.window(p, list);
        let len = self.len(list);
        let h = match p.thumb_h {
            Some(h) => h,
            None if len > page => (bar.h * page as i32 / len as i32).max(8),
            None => return None,
        }
        .min(bar.h);
        let y = bar.y + if len > page { (bar.h - h) * scroll as i32 / (len - page) as i32 } else { 0 };
        Some(rect(bar.x, y, bar.w, h))
    }

    fn scroll_thumb_to(&mut self, p: &Parts, list: List, top: i32) {
        let (Some(bar), Some(thumb)) = (p.bar(list), self.thumb(p, list)) else { return };
        let max = self.len(list).saturating_sub(self.window(p, list).1);
        let track = (bar.h - thumb.h).max(1);
        let t = (top - bar.y).clamp(0, track) as usize;
        self.scroll[slot(list)] = (2 * t * max + track as usize) / (2 * track as usize);
    }

    /// The thumbs the skin draws, each with whether the mouse holds it.
    pub fn thumbs(&self, at: Place) -> Vec<(Rect, bool)> {
        if at.thumb.is_none() {
            return Vec::new();
        }
        let p = Parts::new(at, false);
        [List::Views, List::Tracks]
            .into_iter()
            .filter_map(|l| Some((self.thumb(&p, l)?, self.dragging.is_some_and(|d| d.0 == l))))
            .collect()
    }

    fn show_row(&mut self, p: &Parts, list: List, row: usize) {
        let (scroll, page) = self.window(p, list);
        self.scroll[slot(list)] = if row < scroll {
            row
        } else if row >= scroll + page {
            row + 1 - page
        } else {
            scroll
        };
    }

    fn row_at(&self, p: &Parts, list: List, x: i32, y: i32) -> Option<usize> {
        let rows = p.rows(list).filter(|r| r.contains(x, y))?;
        let n = self.window(p, list).0 + p.row(rows, y);
        (n < self.len(list)).then_some(n)
    }

    // --- columns --------------------------------------------------------------

    /// The two columns across `rows`: the name, and the date when there is room.
    fn columns(rows: Rect, px: i32) -> Vec<(SortBy, i32, i32)> {
        let date = rows.w >= px * 22;
        let cols: Vec<(SortBy, Width)> =
            if date { vec![(SortBy::Name, Width::Share(1)), (SortBy::Date, Width::Px(10))] } else { vec![(SortBy::Name, Width::Share(1))] };
        lay(&cols, rows, px)
    }

    fn heading(by: SortBy) -> &'static str {
        match by {
            SortBy::Name => "Title",
            SortBy::Date => "Date modified",
        }
    }

    // --- drawing --------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, b: &mut Canvas, font: &mut Font, at: Place, c: &PlColors, active: bool, playing: Option<&str>) {
        let (r, px) = (at.r, at.px);
        let p = Parts::new(at, false);
        let (dim, line) = (mix(c.normal, c.normal_bg, 0.45), mix(c.normal, c.normal_bg, 0.7));
        let mut pen = Pen { b, font, px: px as u32, dim, line };
        pen.b.fill(r.x, r.y, r.w, r.h, c.normal_bg);
        pen.b.fill(p.views.x + p.views.w + p.bar_w + 1, p.views.y, 1, p.views.h, line);

        // The folders added, at the left.
        let (scroll, page) = self.window(&p, List::Views);
        for (n, root) in self.roots.iter().enumerate().skip(scroll).take(page) {
            let row = rect(p.views.x, p.views.y + (n - scroll) as i32 * p.row_h, p.views.w, p.row_h);
            let on = self.root == Some(n);
            if on {
                let shade = if active && self.focus == Some(List::Views) { c.selected_bg } else { mix(c.selected_bg, c.normal_bg, 0.5) };
                pen.b.fill(row.x, row.y, row.w, row.h, shade);
            }
            pen.left(row, &label(root), if on { c.current } else { c.normal });
        }
        if self.roots.is_empty() {
            pen.left(rect(p.views.x, p.views.y, p.views.w, p.row_h), "No folders yet", dim);
        }

        // Where the right list is: the folder on screen, in the place of a search field.
        let s = p.search;
        pen.b.fill(s.x, s.y, s.w, s.h, line);
        pen.b.fill(s.x + 1, s.y + 1, s.w - 2, s.h - 2, c.normal_bg);
        let path = self.dir.as_ref().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
        pen.left(rect(s.x + 1, s.y + 1, s.w - 2, s.h - 2), &path, dim);

        self.draw_entries(&mut pen, &p, c, active, playing);
        for list in [List::Views, List::Tracks] {
            let Some(bar) = p.bar(list).filter(|b| b.h > 0) else { continue };
            pen.b.fill(bar.x, bar.y, bar.w, bar.h, mix(c.normal, c.normal_bg, 0.88));
            if let Some(t) = self.thumb(&p, list).filter(|_| at.thumb.is_none()) {
                let held = self.dragging.is_some_and(|d| d.0 == list);
                pen.b.fill(t.x + 1, t.y, t.w - 2, t.h, if held { pen.dim } else { pen.line });
            }
        }

        for (i, (name, _)) in BUTTONS.iter().enumerate() {
            let k = p.buttons[i];
            let face = rect(k.x, k.y + 2, k.w, k.h - 2);
            pen.b.fill(face.x, face.y, face.w, face.h, if self.pressed == Some(i) { c.selected_bg } else { line });
            if self.pressed != Some(i) {
                pen.b.fill(face.x + 1, face.y + 1, face.w - 2, face.h - 2, c.normal_bg);
            }
            pen.centre(face, name, c.normal);
        }
        if p.status.w >= px * 4 {
            pen.right(rect(p.status.x, p.status.y + 2, p.status.w, p.status.h - 2), &self.status(), dim);
        }
    }

    fn draw_entries(&self, pen: &mut Pen, p: &Parts, c: &PlColors, active: bool, playing: Option<&str>) {
        let t = p.table;
        let cols = Self::columns(t.rows, pen.px as i32);
        for &(by, x, w) in &cols {
            let cell = rect(x, t.header.y, w, t.header.h);
            let text = Self::heading(by);
            // The date sits at the right edge, whatever the width of the window.
            if by == SortBy::Date {
                pen.right(cell, text, pen.dim);
            } else {
                pen.left(cell, text, pen.dim);
            }
            // An arrow by the name of the column the list is ordered by: up for A to Z or oldest first.
            // After a name at the left, before the date's name at the right, which stays where it is.
            if let Some((_, reverse)) = self.sort.filter(|s| s.0 == by) {
                let up = reverse == (by == SortBy::Date);
                let tw = pen.width(text);
                let ax = if by == SortBy::Date { (x + w - 10 - tw).max(x + 1) } else { (x + 6 + tw).min(x + w - 6) };
                let mid = t.header.y + t.header.h / 2;
                for k in 0..3 {
                    let y = if up { mid + 1 - k } else { mid - 1 + k };
                    pen.b.fill(ax + 2 - k, y, 1 + 2 * k, 1, pen.dim);
                }
            }
        }
        pen.b.fill(t.header.x, t.header.y + t.header.h - 1, t.header.w, 1, pen.line);
        pen.b.fill(t.rows.x + t.rows.w + p.bar_w + 1, t.header.y, 1, t.header.h + t.rows.h, pen.line);

        if self.roots.is_empty() {
            pen.centre(rect(t.rows.x, t.rows.y + p.row_h, t.rows.w, p.row_h), "No folder is added.", pen.dim);
            pen.centre(rect(t.rows.x, t.rows.y + 2 * p.row_h, t.rows.w, p.row_h), "Press Add folder.", pen.dim);
            return;
        }
        if self.unreadable {
            pen.centre(rect(t.rows.x, t.rows.y + p.row_h, t.rows.w, p.row_h), "The folder cannot be read.", pen.dim);
            return;
        }
        let (scroll, page) = self.window(p, List::Tracks);
        for n in scroll..self.entries.len().min(scroll + page) {
            let e = &self.entries[n];
            let row = rect(t.rows.x, t.rows.y + (n - scroll) as i32 * p.row_h, t.rows.w, p.row_h);
            if self.selected[n] {
                pen.b.fill(row.x, row.y, row.w, row.h, c.selected_bg);
            }
            let color = if playing == Some(e.path.as_str()) { c.current } else { c.normal };
            for &(by, x, w) in &cols {
                let cell = rect(x, row.y, w, row.h);
                match by {
                    SortBy::Name => {
                        // A folder has a small folder in front of its name, a
                        // song a small ball under the folder's middle.
                        let icon = pen.px as i32 + 6;
                        let fx = cell.x + 5;
                        if e.dir && !e.up {
                            let fy = row.y + (row.h - 6) / 2;
                            pen.b.fill(fx, fy, 3, 1, color);
                            pen.b.fill(fx, fy + 1, 8, 5, color);
                        } else if !e.dir {
                            let by = row.y + (row.h - 5) / 2;
                            pen.b.fill(fx + 3, by, 3, 1, color);
                            pen.b.fill(fx + 2, by + 1, 5, 3, color);
                            pen.b.fill(fx + 3, by + 4, 3, 1, color);
                        }
                        pen.left(rect(cell.x + icon, cell.y, (cell.w - icon).max(0), cell.h), &e.name, color);
                    }
                    SortBy::Date => pen.right(cell, &e.modified.map(date_text).unwrap_or_default(), pen.dim),
                }
            }
            if n == self.cursor && active && self.focus.unwrap_or(List::Tracks) == List::Tracks {
                for x in (row.x..row.x + row.w).step_by(2) {
                    pen.b.set(x, row.y, color);
                    pen.b.set(x, row.y + row.h - 1, color);
                }
            }
        }
    }

    // --- mouse ----------------------------------------------------------------

    pub fn press_right(&mut self, at: Place, x: i32, y: i32) {
        let p = Parts::new(at, false);
        if let Some(n) = self.row_at(&p, List::Tracks, x, y).filter(|&n| !self.selected[n]) {
            self.select_only(n);
        } else if let Some(n) = self.row_at(&p, List::Views, x, y) {
            self.focus = Some(List::Views);
            self.pick_root(n);
        }
    }

    fn select_only(&mut self, n: usize) {
        self.selected.fill(false);
        self.selected[n] = true;
        (self.cursor, self.anchor) = (n, n);
    }

    fn select_range(&mut self, to: usize) {
        let (a, b) = (self.anchor.min(to), self.anchor.max(to));
        for (i, on) in self.selected.iter_mut().enumerate() {
            *on = i >= a && i <= b;
        }
        self.cursor = to;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn press(&mut self, at: Place, x: i32, y: i32, shift: bool, ctrl: bool, double: bool) -> Reaction {
        self.carry = None;
        let p = Parts::new(at, false);
        if let Some(i) = p.buttons.iter().position(|k| k.contains(x, y)) {
            self.pressed = Some(i);
            return Reaction { capture: true, run: None };
        }
        if let Some(list) = [List::Views, List::Tracks].into_iter().find(|&l| p.bar(l).is_some_and(|b| b.contains(x, y))) {
            let Some(t) = self.thumb(&p, list) else { return Reaction::default() };
            let grab = if y >= t.y && y < t.y + t.h { y - t.y } else { t.h / 2 };
            self.dragging = Some((list, grab));
            self.scroll_thumb_to(&p, list, y - grab);
            return Reaction { capture: true, run: None };
        }
        if let Some(n) = self.row_at(&p, List::Views, x, y) {
            self.focus = Some(List::Views);
            if self.root != Some(n) {
                self.pick_root(n);
            }
            return Reaction::default();
        }
        self.press_entries(&p, at.px, (x, y), shift, ctrl, double)
    }

    /// A press in the right list: its header sorts, its rows select, a
    /// double click on a folder goes in and on a file opens it.
    fn press_entries(&mut self, p: &Parts, px: i32, (x, y): (i32, i32), shift: bool, ctrl: bool, double: bool) -> Reaction {
        if p.table.header.contains(x, y) {
            if let Some(&(by, ..)) = Self::columns(p.table.rows, px).iter().find(|&&(_, cx, cw)| x >= cx && x < cx + cw) {
                self.sort_by(by);
            }
            return Reaction::default();
        }
        let Some(n) = self.row_at(p, List::Tracks, x, y) else {
            if p.table.rows.contains(x, y) {
                self.selected.fill(false);
            }
            return Reaction::default();
        };
        self.focus = Some(List::Tracks);
        match (double, shift, ctrl) {
            (true, ..) => {
                self.select_only(n);
                return self.open(n);
            }
            (_, true, _) => self.select_range(n),
            (_, _, true) => {
                self.selected[n] = !self.selected[n];
                (self.cursor, self.anchor) = (n, n);
            }
            _ => {
                if !self.selected[n] {
                    self.select_only(n);
                }
                // Not `..`: there is nothing to carry.
                if !self.entries[n].up {
                    self.carry = Some(Carry::Pressed { x, y, row: n });
                    return Reaction { capture: true, run: None };
                }
            }
        }
        Reaction::default()
    }

    /// Opens row `n`: a folder is gone into, a file played or enqueued.
    fn open(&mut self, n: usize) -> Reaction {
        let Some(e) = self.entries.get(n).cloned() else { return Reaction::default() };
        if e.dir {
            self.enter(&e.path);
            Reaction::default()
        } else {
            Reaction { capture: false, run: Some(Command::LibraryOpen) }
        }
    }

    pub fn drag(&mut self, at: Place, x: i32, y: i32) -> bool {
        if let Some((list, grab)) = self.dragging {
            self.scroll_thumb_to(&Parts::new(at, false), list, y - grab);
            return true;
        }
        let Some(Carry::Pressed { x: px, y: py, .. }) = self.carry else { return false };
        let moved = (x - px).abs().max((y - py).abs()) >= CARRY_SLACK;
        if moved {
            self.carry = Some(Carry::Moving);
        }
        moved
    }

    pub fn carrying(&self) -> bool {
        self.carry == Some(Carry::Moving)
    }

    pub fn take_carried(&mut self) -> bool {
        let moving = self.carry == Some(Carry::Moving);
        if moving {
            self.carry = None;
        }
        moving
    }

    pub fn release(&mut self, at: Place, x: i32, y: i32) -> Option<Command> {
        self.dragging = None;
        if let Some(Carry::Pressed { row, .. }) = self.carry.take() {
            self.select_only(row);
        }
        let i = self.pressed.take()?;
        Parts::new(at, false).buttons[i].contains(x, y).then_some(BUTTONS[i].1)
    }

    pub fn wheel(&mut self, at: Place, x: i32, y: i32, steps: i32) {
        let p = Parts::new(at, false);
        let over = [p.rows(List::Views), p.bar(List::Views)].into_iter().flatten().any(|r| r.contains(x, y));
        let list = if over { List::Views } else { List::Tracks };
        let (scroll, page) = self.window(&p, list);
        let max = self.len(list).saturating_sub(page);
        self.scroll[slot(list)] = scroll.saturating_add_signed(-steps as isize * 3).min(max);
    }

    // --- keys -----------------------------------------------------------------

    /// Tab goes between the lists, the arrows move in the one with the
    /// focus, Enter opens (`shift`: the other way), Backspace goes up.
    pub fn key(&mut self, at: Place, key: Key, shift: bool) -> (bool, Option<Command>) {
        let p = Parts::new(at, false);
        let list = self.focus.unwrap_or(List::Tracks);
        match key {
            Key::Enter => {
                let cursor = self.cursor;
                let single_folder = list == List::Tracks && self.entries.get(cursor).is_some_and(|e| e.dir);
                if single_folder && !self.selected.iter().enumerate().any(|(i, &on)| on && i != cursor) {
                    return (true, self.open(cursor).run);
                }
                return (true, Some(if shift { Command::LibraryOpenOther } else { Command::LibraryOpen }));
            }
            Key::Backspace => self.up(),
            Key::Tab => self.focus = Some(if list == List::Tracks { List::Views } else { List::Tracks }),
            Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home | Key::End => self.navigate(&p, list, key, shift),
            _ => return (false, None),
        }
        (true, None)
    }

    fn navigate(&mut self, p: &Parts, list: List, key: Key, shift: bool) {
        let (len, page) = (self.len(list), self.window(p, list).1);
        if len == 0 {
            return;
        }
        let cur = if list == List::Tracks { self.cursor } else { self.root.unwrap_or(0) };
        let to = match key {
            Key::Up => cur.saturating_sub(1),
            Key::Down => cur + 1,
            Key::PageUp => cur.saturating_sub(page),
            Key::PageDown => cur + page,
            Key::Home => 0,
            _ => len - 1,
        }
        .min(len - 1);
        match list {
            List::Tracks if shift => self.select_range(to),
            List::Tracks => self.select_only(to),
            _ => self.pick_root(to),
        }
        self.show_row(p, list, to);
    }

    pub fn select_all(&mut self) {
        for (on, e) in self.selected.iter_mut().zip(&self.entries) {
            *on = !e.up;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = rect(0, 0, 500, 300);
    const PX: i32 = 10;
    const AT: Place = Place { r: R, px: PX, thumb: Some((8, 18)) };

    /// A music folder in a folder of this test's own:
    /// `Albums/` holding `b.mp3` and a folder `Live/`, plus `a.mp3`, `Z.MP3`,
    /// a note and a hidden file.
    fn music(name: &str) -> (PathBuf, FolderTree) {
        let root = std::env::temp_dir().join(format!("popamp-foldertree-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Albums/Live")).unwrap();
        for f in ["a.mp3", "Z.MP3", "notes.txt", ".hidden.mp3", "Albums/b.mp3", "Albums/Live/c.flac"] {
            std::fs::write(root.join(f), b"x").unwrap();
        }
        let mut tree = FolderTree::default();
        assert!(tree.add_root(&root.to_string_lossy()));
        (root, tree)
    }

    fn names(t: &FolderTree) -> Vec<&str> {
        t.entries.iter().map(|e| e.name.as_str()).collect()
    }

    fn row(p: &Parts, n: usize) -> (i32, i32) {
        (p.table.rows.x + 20, p.table.rows.y + n as i32 * p.row_h + 2)
    }

    #[test]
    fn playlist_files_are_listed_with_their_extension_and_open_as_their_songs() {
        let (root, _) = music("lists");
        std::fs::write(root.join("mix.m3u"), "x.ogg
y.ogg
").unwrap();
        std::fs::write(root.join("radio.pls"), "[playlist]
").unwrap();
        std::fs::write(root.join("notes.txt"), "").unwrap();
        let mut t = FolderTree::default();
        t.add_root(&root.to_string_lossy());
        let names: Vec<_> = t.entries.iter().map(|e| e.name.clone()).collect();
        assert!(names.contains(&"mix.m3u".to_string()) && names.contains(&"radio.pls".to_string()));
        assert!(!names.iter().any(|n| n.contains("notes")));
        // Opened, the playlist file goes to the same place a song does, and
        // adding expands it into its songs.
        let i = t.entries.iter().position(|e| e.name == "mix.m3u").unwrap();
        assert_eq!(t.open(i).run, Some(Command::LibraryOpen));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_folder_shows_its_folders_first_then_its_audio_files_without_the_extension() {
        let (root, t) = music("list");
        assert_eq!(names(&t), ["Albums", "a", "Z"], "no note, no hidden file, no .mp3");
        assert!(t.entries[0].dir && !t.entries[1].dir);
        assert_eq!(t.status(), "3 items");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_double_click_goes_into_a_folder_and_dot_dot_comes_back() {
        let (root, mut t) = music("enter");
        let p = Parts::new(AT, false);
        let (x, y) = row(&p, 0);
        assert_eq!(t.press(AT, x, y, false, false, true), Reaction::default(), "a folder opens in the list");
        assert_eq!(names(&t), ["..", "Live", "b"]);
        assert_eq!(t.current(), Some(root.join("Albums").as_path()));
        let (x, y) = row(&p, 1);
        t.press(AT, x, y, false, false, true);
        assert_eq!(names(&t), ["..", "c"]);
        let (x, y) = row(&p, 0);
        t.press(AT, x, y, false, false, true);
        assert_eq!(names(&t), ["..", "Live", "b"], "up one folder");
        assert_eq!(t.key(AT, Key::Backspace, false), (true, None));
        assert_eq!(names(&t), ["Albums", "a", "Z"], "Backspace goes up too, and the picked folder has no ..");
        assert_eq!(t.key(AT, Key::Backspace, false), (true, None));
        assert_eq!(names(&t).len(), 3, "it stays in the picked folder");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_double_click_on_a_file_plays_or_enqueues_it() {
        let (root, mut t) = music("open");
        let p = Parts::new(AT, false);
        let (x, y) = row(&p, 1);
        let re = t.press(AT, x, y, false, false, true);
        assert_eq!(re.run, Some(Command::LibraryOpen));
        assert_eq!(t.picked(), [root.join("a.mp3").to_string_lossy().into_owned()]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_headers_sort_and_a_second_click_turns_the_order_round() {
        let (root, mut t) = music("sort");
        let p = Parts::new(AT, false);
        let name_header = (p.table.header.x + 5, p.table.header.y + 2);
        assert_eq!(t.sort, None, "no arrow until a header is pressed");
        t.press(AT, name_header.0, name_header.1, false, false, false);
        assert_eq!((names(&t), t.sort), (vec!["Albums", "a", "Z"], Some((SortBy::Name, false))), "A to Z first");
        t.press(AT, name_header.0, name_header.1, false, false, false);
        assert_eq!(names(&t), ["Albums", "Z", "a"], "then Z to A, the folder still first");
        t.press(AT, name_header.0, name_header.1, false, false, false);
        assert_eq!(names(&t), ["Albums", "a", "Z"], "A to Z again");
        // Date: the newest first, then the oldest.
        let old = std::fs::File::options().write(true).open(root.join("a.mp3")).unwrap();
        old.set_modified(SystemTime::now() - std::time::Duration::from_secs(86_400 * 30)).unwrap();
        let date = Parts::new(AT, false);
        let cols = FolderTree::columns(date.table.rows, PX);
        let (_, dx, _) = cols[1];
        t.press(AT, dx + 3, date.table.header.y + 2, false, false, false);
        assert_eq!(names(&t), ["Albums", "Z", "a"], "newest first");
        t.press(AT, dx + 3, date.table.header.y + 2, false, false, false);
        assert_eq!(names(&t), ["Albums", "a", "Z"], "oldest first");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn what_is_picked_is_what_plays() {
        let (root, mut t) = music("picked");
        let all = |t: &FolderTree| t.picked().iter().map(|p| label(p)).collect::<Vec<_>>();
        assert_eq!(all(&t), ["a.mp3", "Z.MP3"], "none picked: the files shown");
        let p = Parts::new(AT, false);
        let (x, y) = row(&p, 0);
        t.press(AT, x, y, false, false, false);
        t.release(AT, x, y);
        assert_eq!(all(&t), ["Albums"], "a folder is passed on whole");
        let (x, y) = row(&p, 2);
        t.press(AT, x, y, true, false, false);
        assert_eq!(all(&t), ["Albums", "a.mp3", "Z.MP3"], "shift picks the range");
        t.select_all();
        assert_eq!(all(&t).len(), 3);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_left_list_keeps_the_added_folders_and_a_click_shows_one() {
        let (root, mut t) = music("roots");
        let other = root.join("Albums");
        assert!(t.add_root(&other.to_string_lossy()));
        assert!(!t.add_root(&other.to_string_lossy()), "not twice");
        assert_eq!(names(&t), ["Live", "b"], "the folder just added is shown");
        let p = Parts::new(AT, false);
        t.press(AT, p.views.x + 5, p.views.y + 2, false, false, false);
        assert_eq!(names(&t), ["Albums", "a", "Z"], "the first one again");
        let file = root.join("foldertree.txt");
        t.save(&file).unwrap();
        let back = FolderTree::load(&file);
        assert_eq!(back.roots, t.roots);
        assert_eq!(names(&back), ["Albums", "a", "Z"], "the first is shown at start");
        t.remove_root();
        assert_eq!(t.roots.len(), 1);
        assert_eq!(names(&t), ["Live", "b"]);
        t.remove_root();
        assert!(t.roots.is_empty() && t.entries.is_empty() && t.folder().is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_buttons_run_their_commands_and_add_folder_is_right_of_rescan() {
        assert_eq!(
            BUTTONS.map(|b| b.1),
            [Command::LibraryPlay, Command::LibraryEnqueue, Command::LibraryRescan, Command::LibraryAddFolder]
        );
        let (root, mut t) = music("buttons");
        let p = Parts::new(AT, false);
        let add = p.buttons[3];
        assert!(add.x > p.buttons[2].x, "to the right of Rescan");
        let (x, y) = (add.x + 3, add.y + 3);
        assert!(t.press(AT, x, y, false, false, false).capture);
        assert_eq!(t.release(AT, x, y), Some(Command::LibraryAddFolder));
        assert_eq!(t.release(AT, 0, 0), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_keys_move_open_and_go_up() {
        let (root, mut t) = music("keys");
        assert_eq!(t.key(AT, Key::Down, false), (true, None));
        assert_eq!(t.key(AT, Key::Up, false), (true, None));
        assert_eq!(t.key(AT, Key::Enter, false), (true, None), "Enter on a folder goes in");
        assert_eq!(names(&t), ["..", "Live", "b"]);
        t.key(AT, Key::Down, false);
        t.key(AT, Key::Down, false);
        assert_eq!(t.key(AT, Key::Enter, false), (true, Some(Command::LibraryOpen)), "on a file it plays");
        assert_eq!(t.key(AT, Key::Enter, true), (true, Some(Command::LibraryOpenOther)));
        assert_eq!(t.key(AT, Key::Char('x'), false), (false, None), "no search here");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_model_adds_removes_and_keeps_the_folders_of_the_tree() {
        let mut m = crate::model::Model::for_tests();
        let (root, _) = music("model");
        m.set_library_type(true);
        assert!(m.library.tree_mode);
        m.library_add_folder(&root.to_string_lossy());
        assert_eq!(m.library.folder(), Some(root.to_string_lossy().as_ref()));
        assert_eq!(m.library.picked().len(), 2, "no tags read: the two files by name");
        assert_eq!(m.library.status(), "3 items");
        let saved = std::fs::read_to_string(m.paths.foldertree_file()).unwrap();
        assert_eq!(saved.trim(), root.to_string_lossy());
        m.library_add_folder(&root.to_string_lossy());
        assert_eq!(m.library.tree.roots.len(), 1, "not twice");
        assert!(m.library.lib.folders.is_empty(), "the tag library is left alone");
        m.library_rescan();
        m.exec(Command::LibraryRemoveFolder);
        assert!(m.library.tree.roots.is_empty());
        assert_eq!(std::fs::read_to_string(m.paths.foldertree_file()).unwrap().trim(), "");
        let menu = m.library_menu();
        assert!(menu.iter().any(|i| format!("{i:?}").contains("Add folder")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_folder_has_a_folder_icon_and_a_song_a_ball_in_front_of_its_name() {
        let (root, t) = music("icons");
        let m = crate::model::Model::for_tests();
        let mut fonts = crate::font::Fonts::for_skin(&m.skin);
        let mut bitmap = popamp_skin::Bitmap::new(R.w, R.h, 0);
        t.draw(&mut Canvas::new(&mut bitmap, crate::scale::Scale::ONE), &mut fonts.list, AT, &m.skin.pl, true, None);
        let p = Parts::new(AT, false);
        let (x0, bg, ink) = (p.table.rows.x, m.skin.pl.normal_bg, m.skin.pl.normal);
        let at = |x: i32, y: i32| bitmap.get(x, y).unwrap();
        // The folder row: its icon starts five pixels in; nothing is drawn at three and four.
        let (fy, mid) = (p.table.rows.y + (p.row_h - 6) / 2, p.table.rows.y + p.row_h / 2);
        assert_eq!((at(x0 + 3, mid), at(x0 + 4, mid)), (bg, bg));
        assert_eq!((at(x0 + 5, fy + 3), at(x0 + 12, fy + 3)), (ink, ink), "the folder is 8 wide");
        // The song row: a ball, centred under the folder's middle, 5 across.
        let (row, by) = (p.table.rows.y + p.row_h, p.table.rows.y + p.row_h + (p.row_h - 5) / 2);
        assert_eq!((at(x0 + 7, row + p.row_h / 2), at(x0 + 11, row + p.row_h / 2)), (ink, ink), "5 across at the middle");
        assert_eq!((at(x0 + 6, row + p.row_h / 2), at(x0 + 12, row + p.row_h / 2)), (bg, bg));
        assert_eq!((at(x0 + 9, by), at(x0 + 9, by + 4), at(x0 + 7, by), at(x0 + 11, by)), (ink, ink, bg, bg), "round: no corners");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_dates_and_their_heading_are_flush_right_whatever_the_width() {
        let (root, t) = music("right");
        let m = crate::model::Model::for_tests();
        let mut fonts = crate::font::Fonts::for_skin(&m.skin);
        // The rightmost drawn pixel of a row, between its name column's end and the scroll bar.
        let mut reach = |width: i32, row: usize, head: bool| {
            let at = Place { r: rect(0, 0, width, 300), ..AT };
            let mut b = popamp_skin::Bitmap::new(width, 300, 0);
            t.draw(&mut Canvas::new(&mut b, crate::scale::Scale::ONE), &mut fonts.list, at, &m.skin.pl, true, None);
            let p = Parts::new(at, false);
            let y = if head { p.table.header.y } else { p.table.rows.y + row as i32 * p.row_h };
            let h = if head { p.table.header.h - 2 } else { p.row_h };
            let right = p.table.rows.x + p.table.rows.w;
            let bg = m.skin.pl.normal_bg;
            let mut best = 0;
            for yy in y..y + h {
                for x in (right - 40)..right {
                    if b.get(x, yy).is_some_and(|p| p != bg) {
                        best = best.max(x);
                    }
                }
            }
            (best, right)
        };
        for width in [500, 640] {
            let (row, right) = reach(width, 1, false);
            assert!(row >= right - 5 && row < right, "{width}: a date ends at the right edge, not {} short", right - row);
            let (head, right) = reach(width, 0, true);
            assert!(head >= right - 5 && head < right, "{width}: so does its heading");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dates_are_written_as_explorer_does() {
        let at = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let text = date_text(at);
        assert_eq!(text.len(), 16, "{text}");
        assert!(text.starts_with("2023-11-1"), "{text}");
    }
}
