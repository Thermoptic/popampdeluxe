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
    /// The file's whole name, with its extension.
    pub file: String,
    pub path: String,
    pub dir: bool,
    /// The row `..`, which goes up to the folder above.
    pub up: bool,
    pub modified: Option<SystemTime>,
}

/// A line of the left list: a group, or a folder that may be in a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A heading that folds the folders under it in and out.
    Group { name: String, open: bool },
    /// A folder. `name` is what the list says, or empty for the folder's own
    /// name; `child`: it is in the group above it.
    Folder { path: String, name: String, child: bool },
}

impl Item {
    pub fn is_folder(&self) -> bool {
        matches!(self, Item::Folder { .. })
    }

    /// What the list says.
    pub fn label(&self) -> String {
        match self {
            Item::Group { name, .. } => name.clone(),
            Item::Folder { name, .. } if !name.is_empty() => name.clone(),
            Item::Folder { path, .. } => label(path),
        }
    }

    /// The line it is kept as in the ini file: `group|1|Name` (1: unfolded)
    /// or `folder|1|Name|path` (1: in a group).
    pub fn line(&self) -> String {
        match self {
            Item::Group { name, open } => format!("group|{}|{name}", u8::from(*open)),
            Item::Folder { path, name, child } => format!("folder|{}|{name}|{path}", u8::from(*child)),
        }
    }

    pub fn parse(line: &str) -> Option<Item> {
        let (kind, rest) = line.trim().split_once('|')?;
        match kind {
            "group" => {
                let (open, name) = rest.split_once('|')?;
                Some(Item::Group { name: name.trim().to_string(), open: open != "0" })
            }
            "folder" => {
                let mut parts = rest.splitn(3, '|');
                let (child, name, path) = (parts.next()?, parts.next()?, parts.next()?);
                (!path.trim().is_empty()).then(|| Item::Folder { path: path.trim().to_string(), name: name.trim().to_string(), child: child != "0" })
            }
            _ => None,
        }
    }
}

/// A name as the ini file can keep it: one line, no bars.
pub fn clean_name(name: &str) -> String {
    name.replace(['|', '\n', '\r'], " ").trim().to_string()
}

/// Makes the list hold together: a folder is in a group only when a group
/// (or a folder of it) is just above.
pub fn normalize(items: &mut [Item]) {
    let mut inside = false;
    for it in items {
        match it {
            Item::Group { .. } => inside = true,
            Item::Folder { child, .. } => {
                *child = *child && inside;
                inside = *child;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct FolderTree {
    /// The groups and folders of the left list.
    pub items: Vec<Item>,
    /// The picked folder, by its place in `items`.
    root: Option<usize>,
    /// The row of the left list last clicked (a group too), by its place in
    /// `items`: while that list has the focus, Play and Enqueue take all it holds.
    left: Option<usize>,
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
    dragging: Option<(List, i32)>,
    carry: Option<Carry>,
    /// The folder on screen could not be read.
    unreadable: bool,
    /// Names are written without their extension.
    pub hide_ext: bool,
}

/// Whether `path` is a file Popamp plays.
fn is_audio(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| popamp_core::audio_extensions().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Whether the row is a playlist file.
fn is_list(e: &Entry) -> bool {
    !e.dir && popamp_core::plformats::is_playlist(&e.file)
}

/// The name of the last part of `path`.
pub fn path_label(path: &str) -> String {
    label(path)
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
    /// The folders kept in `file`, one to a line: how they were kept before
    /// the groups, and still read when the ini file has no tree yet.
    pub fn load(file: &Path) -> FolderTree {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        let lines: Vec<String> = text.lines().map(str::trim).filter(|l| !l.is_empty()).map(|l| format!("folder|0||{l}")).collect();
        FolderTree::from_lines(&lines)
    }

    /// The tree kept as `lines` (see [`Item::line`]).
    pub fn from_lines(lines: &[String]) -> FolderTree {
        let mut tree = FolderTree { items: lines.iter().filter_map(|l| Item::parse(l)).collect(), ..FolderTree::default() };
        normalize(&mut tree.items);
        tree.pick_first();
        tree
    }

    pub fn lines(&self) -> Vec<String> {
        self.items.iter().map(Item::line).collect()
    }

    fn pick_first(&mut self) {
        match self.items.iter().position(Item::is_folder) {
            Some(i) => self.pick_root(i),
            None => {
                (self.root, self.dir) = (None, None);
                self.reload();
            }
        }
    }

    /// Whether any folder is added.
    pub fn has_folders(&self) -> bool {
        self.items.iter().any(Item::is_folder)
    }

    /// The paths of the folders, in order.
    #[cfg(test)]
    pub fn folders(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Folder { path, .. } => Some(path.as_str()),
                Item::Group { .. } => None,
            })
            .collect()
    }

    /// Adds `folder` to the left list and shows it. False when it is there already.
    pub fn add_root(&mut self, folder: &str) -> bool {
        if let Some(i) = self.items.iter().position(|i| matches!(i, Item::Folder { path, .. } if path == folder)) {
            self.pick_root(i);
            return false;
        }
        self.items.push(Item::Folder { path: folder.to_string(), name: String::new(), child: false });
        self.pick_root(self.items.len() - 1);
        true
    }

    /// Takes the picked folder out of the left list; its files stay.
    pub fn remove_root(&mut self) {
        let Some(i) = self.root else { return };
        self.left = None;
        self.items.remove(i);
        normalize(&mut self.items);
        self.scroll = [0, 0];
        let next = (i..self.items.len()).chain((0..i.min(self.items.len())).rev()).find(|&n| self.items[n].is_folder());
        match next {
            Some(n) => self.pick_root(n),
            None => self.pick_first(),
        }
    }

    /// Puts `items` in the place of the list, keeping the folder on screen
    /// when it is still there.
    pub fn set_items(&mut self, mut items: Vec<Item>) {
        normalize(&mut items);
        let on = self.folder().map(String::from);
        self.left = None;
        self.items = items;
        let found = on.and_then(|p| self.items.iter().position(|i| matches!(i, Item::Folder { path, .. } if *path == p)));
        match found {
            Some(i) => {
                self.root = Some(i);
                self.reload();
            }
            None => self.pick_first(),
        }
    }

    /// A row of the left list was picked: a group folds in or out, a folder shows.
    fn pick_row(&mut self, n: usize) {
        let Some(&i) = self.shown().get(n) else { return };
        self.left = Some(i);
        match self.items[i] {
            Item::Group { ref mut open, .. } => *open = !*open,
            Item::Folder { .. } if self.root != Some(i) => self.pick_root(i),
            Item::Folder { .. } => {}
        }
    }

    /// The places in `items` of the rows on screen: a folded group hides
    /// the folders in it.
    pub fn shown(&self) -> Vec<usize> {
        let mut open = true;
        let mut rows = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            match it {
                Item::Group { open: o, .. } => {
                    open = *o;
                    rows.push(i);
                }
                Item::Folder { child: false, .. } => {
                    open = true;
                    rows.push(i);
                }
                Item::Folder { .. } if open => rows.push(i),
                Item::Folder { .. } => {}
            }
        }
        rows
    }

    /// The folder picked in the left list.
    pub fn folder(&self) -> Option<&str> {
        match self.items.get(self.root?)? {
            Item::Folder { path, .. } => Some(path),
            Item::Group { .. } => None,
        }
    }

    fn pick_root(&mut self, i: usize) {
        let Some(Item::Folder { path, .. }) = self.items.get(i) else { return };
        self.dir = Some(PathBuf::from(path));
        self.root = Some(i);
        self.scroll[1] = 0;
        self.reload();
    }

    /// The folder on screen: the picked folder, or one inside it.
    #[cfg(test)]
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
                        let file = name.clone();
                        let name = if meta.is_dir() {
                            name
                        } else if is_audio(&path) || popamp_core::plformats::is_playlist(&name) {
                            path.file_stem().map_or(name, |s| s.to_string_lossy().into_owned())
                        } else {
                            continue;
                        };
                        let path = path.to_string_lossy().into_owned();
                        self.entries.push(Entry { name, file, path, dir: meta.is_dir(), up: false, modified: meta.modified().ok() });
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
            let up = Entry { name: "..".into(), file: "..".into(), path: parent.to_string_lossy().into_owned(), dir: true, up: true, modified: None };
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
        // With the left list in hand: the folder clicked there, all of it, or
        // the folders of the group clicked.
        if self.focus == Some(List::Views) {
            match self.left.and_then(|i| self.items.get(i).map(|it| (i, it))) {
                Some((_, Item::Folder { path, .. })) => return vec![path.clone()],
                Some((i, Item::Group { .. })) => {
                    return self.items[i + 1..]
                        .iter()
                        .map_while(|it| match it {
                            Item::Folder { path, child: true, .. } => Some(path.clone()),
                            _ => None,
                        })
                        .collect();
                }
                None => {}
            }
        }
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
        if list == List::Views { self.shown().len() } else { self.entries.len() }
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

        // The groups and folders added, at the left.
        let (scroll, page) = self.window(&p, List::Views);
        for (n, &i) in self.shown().iter().enumerate().skip(scroll).take(page) {
            let row = rect(p.views.x, p.views.y + (n - scroll) as i32 * p.row_h, p.views.w, p.row_h);
            let item = &self.items[i];
            let on = self.root == Some(i);
            if on {
                let shade = if active && self.focus == Some(List::Views) { c.selected_bg } else { mix(c.selected_bg, c.normal_bg, 0.5) };
                pen.b.fill(row.x, row.y, row.w, row.h, shade);
            }
            let color = if on { c.current } else { c.normal };
            let icon = pen.px as i32 + 6;
            let (fx, fy) = (row.x + 5, row.y + (row.h - 6) / 2);
            match item {
                Item::Group { .. } => {
                    // An empty folder: the group's own picture.
                    pen.b.fill(fx, fy, 3, 1, color);
                    pen.b.fill(fx, fy + 1, 8, 1, color);
                    pen.b.fill(fx, fy + 2, 1, 3, color);
                    pen.b.fill(fx + 7, fy + 2, 1, 3, color);
                    pen.b.fill(fx, fy + 5, 8, 1, color);
                }
                Item::Folder { child: false, .. } => {
                    pen.b.fill(fx, fy, 3, 1, color);
                    pen.b.fill(fx, fy + 1, 8, 5, color);
                }
                Item::Folder { child: true, .. } => {
                    // An L: this folder hangs under the group above.
                    pen.b.fill(fx + 3, fy, 1, 3, color);
                    pen.b.fill(fx + 3, fy + 3, 4, 1, color);
                }
            }
            pen.left(rect(row.x + icon, row.y, (row.w - icon).max(0), row.h), &item.label(), color);
        }
        if self.items.is_empty() {
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

        if !self.has_folders() {
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
                        } else if is_list(e) {
                            // A playlist: a small page with lines on it.
                            let py = row.y + (row.h - 7) / 2;
                            let px = fx + 1;
                            pen.b.fill(px, py, 5, 1, color);
                            pen.b.fill(px, py + 1, 1, 5, color);
                            pen.b.fill(px + 5, py + 1, 1, 5, color);
                            pen.b.fill(px, py + 6, 6, 1, color);
                            pen.b.set(px + 2, py + 2, color);
                            pen.b.fill(px + 2, py + 4, 2, 1, color);
                        } else if !e.dir {
                            let by = row.y + (row.h - 5) / 2;
                            pen.b.fill(fx + 3, by, 3, 1, color);
                            pen.b.fill(fx + 2, by + 1, 5, 3, color);
                            pen.b.fill(fx + 3, by + 4, 3, 1, color);
                        }
                        let shown = if self.hide_ext { &e.name } else { &e.file };
                        pen.left(rect(cell.x + icon, cell.y, (cell.w - icon).max(0), cell.h), shown, color);
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
        if let Some(n) = self.row_at(&p, List::Tracks, x, y) {
            self.focus = Some(List::Tracks);
            if !self.selected[n] {
                self.select_only(n);
            }
        } else if let Some(n) = self.row_at(&p, List::Views, x, y) {
            // A folder (or group) of the left list: the menu is for all it holds.
            self.focus = Some(List::Views);
            if let Some(&i) = self.shown().get(n) {
                self.left = Some(i);
                self.pick_root(i);
            }
        }
    }

    /// The rows of the right list that the jump window searches: the place
    /// of each in the list, and how it is written (a folder in brackets).
    pub fn jump_rows(&self) -> Vec<(usize, String)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.up)
            .map(|(n, e)| {
                let name = if self.hide_ext { &e.name } else { &e.file };
                (n, if e.dir { format!("[{name}]") } else { name.clone() })
            })
            .collect()
    }

    /// Marks row `n` of the right list, a folder or a file, as the only one
    /// picked, and scrolls to it. Nothing is opened or played.
    pub fn jump_to(&mut self, n: usize) {
        if n >= self.entries.len() {
            return;
        }
        self.select_only(n);
        self.focus = Some(List::Tracks);
        self.scroll[slot(List::Tracks)] = n.saturating_sub(2);
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
        if let Some(list) = [List::Views, List::Tracks].into_iter().find(|&l| p.bar(l).is_some_and(|b| b.contains(x, y))) {
            let Some(t) = self.thumb(&p, list) else { return Reaction::default() };
            let grab = if y >= t.y && y < t.y + t.h { y - t.y } else { t.h / 2 };
            self.dragging = Some((list, grab));
            self.scroll_thumb_to(&p, list, y - grab);
            return Reaction { capture: true, run: None };
        }
        if let Some(n) = self.row_at(&p, List::Views, x, y) {
            self.focus = Some(List::Views);
            self.pick_row(n);
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

    pub fn release(&mut self, _at: Place, _x: i32, _y: i32) -> Option<Command> {
        self.dragging = None;
        if let Some(Carry::Pressed { row, .. }) = self.carry.take() {
            self.select_only(row);
        }
        None
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
        let rows = self.shown();
        let cur = if list == List::Tracks { self.cursor } else { rows.iter().position(|&i| Some(i) == self.root).unwrap_or(0) };
        let mut to = match key {
            Key::Up => cur.saturating_sub(1),
            Key::Down => cur + 1,
            Key::PageUp => cur.saturating_sub(page),
            Key::PageDown => cur + page,
            Key::Home => 0,
            _ => len - 1,
        }
        .min(len - 1);
        if list == List::Views {
            self.left = None;
            // A group is no folder to show: go on to the nearest folder.
            let up = to < cur || key == Key::Home;
            let ok = |n: &usize| self.items[rows[*n]].is_folder();
            let found = if up { (0..=to).rev().find(ok) } else { (to..len).find(ok) };
            let Some(n) = found.or_else(|| if up { (to..len).find(ok) } else { (0..=to).rev().find(ok) }) else { return };
            to = n;
            self.left = Some(rows[to]);
            self.pick_root(rows[to]);
        }
        match list {
            List::Tracks if shift => self.select_range(to),
            List::Tracks => self.select_only(to),
            _ => {}
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
    const AT: Place = Place { r: R, px: PX, gap: 3, thumb: Some((8, 18)) };

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

    fn grouped(name: &str) -> (PathBuf, FolderTree) {
        let (root, _) = music(name);
        let at = |d: &str| root.join(d).to_string_lossy().into_owned();
        let lines = vec![
            format!("folder|0||{}", root.to_string_lossy()),
            "group|1|Play".to_string(),
            format!("folder|1|Live|{}", at("Albums/Live")),
            format!("folder|1||{}", at("Albums")),
        ];
        (root, FolderTree::from_lines(&lines))
    }

    #[test]
    fn a_group_holds_folders_and_a_click_on_it_folds_them_in_and_out() {
        let (root, mut t) = grouped("grp-fold");
        assert_eq!(t.shown(), [0, 1, 2, 3], "all shown: the group is unfolded");
        assert_eq!(t.items[2].label(), "Live", "a folder can have a name of its own");
        assert_eq!(t.items[3].label(), "Albums", "else its own name");
        let p = Parts::new(AT, false);
        let on = |n: usize| (p.views.x + 5, p.views.y + n as i32 * p.row_h + 2);
        let shown_before = t.folder().map(String::from);
        let (x, y) = on(1);
        t.press(AT, x, y, false, false, false);
        assert_eq!(t.shown(), [0, 1], "the group is folded in: its folders are hidden");
        assert_eq!(t.folder().map(String::from), shown_before, "the group is no folder: the right list stays");
        t.press(AT, x, y, false, false, false);
        assert_eq!(t.shown(), [0, 1, 2, 3], "and out again");
        let (x, y) = on(2);
        t.press(AT, x, y, false, false, false);
        assert_eq!(names(&t), ["c"], "a folder in the group shows its files");
        assert_eq!(t.lines()[1], "group|1|Play");
        t.items[1] = Item::Group { name: "Play".into(), open: false };
        assert_eq!(t.lines()[1], "group|0|Play", "whether it is folded is kept");
        let back = FolderTree::from_lines(&t.lines());
        assert_eq!(back.items, t.items);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_keys_skip_a_group_and_a_folded_group_hides_its_folders_from_them() {
        let (root, mut t) = grouped("grp-keys");
        t.focus = Some(List::Views);
        t.pick_root(0);
        t.key(AT, Key::Down, false);
        assert_eq!(t.folder().map(label), Some("Live".to_string()), "down passes the group");
        t.key(AT, Key::Up, false);
        assert_eq!(t.folder().map(label), Some(label(&root.to_string_lossy())), "up passes it too");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_group_and_a_folder_in_it_have_their_own_icons() {
        let (root, t) = grouped("grp-icons");
        let mut b = popamp_skin::Bitmap::new(500, 300, 0);
        let mut font = crate::font::Fonts::for_skin(&popamp_skin::Skin::builtin_vector(None)).list;
        let c = PlColors { normal: 0xffffff, normal_bg: 0x000000, current: 0xffffff, selected_bg: 0x222222, font: String::new() };
        t.draw(&mut Canvas::new(&mut b, crate::scale::Scale::ONE), &mut font, AT, &c, true, None);
        let p = Parts::new(AT, false);
        let at = |n: i32, dx: i32, dy: i32| b.get(p.views.x + 5 + dx, p.views.y + n * p.row_h + (p.row_h - 6) / 2 + dy);
        let (ink, bg) = (Some(0xffffff), Some(0));
        // Row 0 a folder (filled), row 1 the group (hollow), row 2 under it (an L).
        assert_eq!((at(0, 4, 3), at(1, 4, 3), at(1, 0, 3), at(1, 7, 3)), (ink, bg, ink, ink), "a hollow folder for the group");
        assert_eq!((at(2, 3, 0), at(2, 3, 3), at(2, 6, 3), at(2, 0, 0)), (ink, ink, ink, bg), "an L for the folder in it");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_jump_window_searches_the_right_list_and_goes_to_what_is_found() {
        let (root, mut t) = music("jump");
        t.hide_ext = true;
        t.reload();
        let rows = t.jump_rows();
        let texts: Vec<&str> = rows.iter().map(|r| r.1.as_str()).collect();
        assert_eq!(texts, ["[Albums]", "a", "Z"], "folders in brackets, no .mp3, no note, no ..");
        // A file: marked as the only one picked; nothing is played.
        let z = rows.iter().find(|r| r.1 == "Z").unwrap().0;
        t.jump_to(z);
        assert_eq!(t.picked().iter().map(|p| label(p)).collect::<Vec<_>>(), ["Z.MP3"]);
        assert_eq!(names(&t), ["Albums", "a", "Z"], "still the same folder");
        // A folder: marked too, not opened.
        let albums = rows[0].0;
        t.jump_to(albums);
        assert_eq!(t.picked().iter().map(|p| label(p)).collect::<Vec<_>>(), ["Albums"]);
        assert_eq!(names(&t), ["Albums", "a", "Z"], "not gone into");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn enqueue_on_a_folder_of_the_left_list_takes_all_it_holds_and_on_a_row_of_the_right_one_that_row() {
        let (root, mut t) = music("left-pick");
        let albums = root.join("Albums");
        let lines = vec![
            format!("folder|0||{}", root.to_string_lossy()),
            "group|1|G".to_string(),
            format!("folder|1||{}", albums.to_string_lossy()),
        ];
        t = FolderTree::from_lines(&lines);
        let p = Parts::new(AT, false);
        let left = |n: i32| (p.views.x + 5, p.views.y + n * p.row_h + 2);
        // Right-click the first folder of the left list: the folder itself, all of it.
        let (x, y) = left(0);
        t.press_right(AT, x, y);
        assert_eq!(t.picked(), [root.to_string_lossy().to_string()], "the whole folder, which expands to every song in it");
        // A group: the folders in it.
        let (x, y) = left(1);
        t.press_right(AT, x, y);
        assert_eq!(t.picked(), [albums.to_string_lossy().to_string()]);
        // A row of the right list: that row, as before.
        t.press_right(AT, p.table.rows.x + 20, p.table.rows.y + p.row_h + 2);
        assert_eq!(t.picked().iter().map(|p| label(p)).collect::<Vec<_>>(), ["a.mp3"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn hide_file_extension_is_a_preference_that_shows_or_hides_every_extension() {
        let mut m = crate::model::Model::for_tests();
        assert!(m.settings().ml_hide_ext && m.library.tree.hide_ext, "hidden by default");
        crate::panels::Flag::HideExt.set(&mut m, false);
        assert!(!m.settings().ml_hide_ext && !m.library.tree.hide_ext);
        crate::panels::Flag::HideExt.set(&mut m, true);
        assert!(m.library.tree.hide_ext);
    }

    #[test]
    fn playlist_files_are_listed_with_their_own_icon_and_open_as_their_songs() {
        let (root, _) = music("lists");
        std::fs::write(root.join("mix.m3u"), "x.ogg
y.ogg
").unwrap();
        std::fs::write(root.join("radio.pls"), "[playlist]
").unwrap();
        std::fs::write(root.join("notes.txt"), "").unwrap();
        let mut t = FolderTree::default();
        t.add_root(&root.to_string_lossy());
        let names: Vec<_> = t.entries.iter().map(|e| e.file.clone()).collect();
        assert!(names.contains(&"mix.m3u".to_string()) && names.contains(&"radio.pls".to_string()));
        assert!(!names.iter().any(|n| n.contains("notes")));
        assert!(t.entries.iter().any(|e| e.name == "mix" && is_list(e)), "the name without its extension, and a playlist icon");
        // Opened, the playlist file goes to the same place a song does, and
        // adding expands it into its songs.
        let i = t.entries.iter().position(|e| e.file == "mix.m3u").unwrap();
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
        let back = FolderTree::from_lines(&t.lines());
        assert_eq!(back.folders(), t.folders());
        let file = root.join("foldertree.txt");
        std::fs::write(&file, t.folders().join("
")).unwrap();
        assert_eq!(FolderTree::load(&file).folders(), t.folders(), "the old file is still read");
        assert_eq!(names(&back), ["Albums", "a", "Z"], "the first is shown at start");
        t.remove_root();
        assert_eq!(t.folders().len(), 1);
        assert_eq!(names(&t), ["Live", "b"]);
        t.remove_root();
        assert!(t.items.is_empty() && t.entries.is_empty() && t.folder().is_none());
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
        let saved = m.config.folder_tree().expect("kept in the ini file");
        assert_eq!(saved, [format!("folder|0||{}", root.to_string_lossy())]);
        m.library_add_folder(&root.to_string_lossy());
        assert_eq!(m.library.tree.folders().len(), 1, "not twice");
        assert!(m.library.lib.folders.is_empty(), "the tag library is left alone");
        m.library_rescan();
        m.exec(Command::LibraryRemoveFolder);
        assert!(m.library.tree.items.is_empty());
        assert_eq!(m.config.folder_tree(), Some(vec![]), "an empty tree is saved too");
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
