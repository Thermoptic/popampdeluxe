//! What was played, newest last, kept in `history.txt`.
//!
//! Each play is one appended line, `secs<TAB>location<TAB>title`, so a
//! crash loses at most the line being written. Loading keeps the newest
//! `limit` plays (Preferences > Interface, [`DEFAULT_LIMIT`] unless changed)
//! and rewrites the file once it holds twice that many.

use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;

use crate::text::{escape_field as escape, unescape_field as unescape};

/// How many plays are kept unless the setting says otherwise.
pub const DEFAULT_LIMIT: usize = 800;

/// One track started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    /// Seconds since the Unix epoch.
    pub at: u64,
    pub location: String,
    /// The title the playlist showed then.
    pub title: String,
}

impl Played {
    fn line(&self) -> String {
        format!("{}\t{}\t{}\n", self.at, escape(&self.location), escape(&self.title))
    }

    fn parse(line: &str) -> Option<Played> {
        let mut parts = line.splitn(3, '\t');
        let at = parts.next()?.parse().ok()?;
        let location = unescape(parts.next().filter(|l| !l.is_empty())?);
        let title = unescape(parts.next()?);
        Some(Played { at, location, title })
    }
}

#[derive(Debug)]
pub struct History {
    plays: VecDeque<Played>,
    /// How many plays are kept.
    limit: usize,
}

impl Default for History {
    fn default() -> History {
        History { plays: VecDeque::new(), limit: DEFAULT_LIMIT }
    }
}

impl History {
    /// The newest `limit` plays in `text`; broken lines are skipped.
    pub fn parse(text: &str, limit: usize) -> History {
        let mut h = History { plays: VecDeque::new(), limit: limit.max(1) };
        text.lines().filter_map(Played::parse).for_each(|p| h.push(p));
        h
    }

    /// Reads `path`, compacting it when it has grown past twice the limit.
    /// A missing file is an empty history.
    pub fn load(path: &Path, limit: usize) -> History {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let h = History::parse(&text, limit);
        if text.lines().count() > 2 * h.limit {
            let all: String = h.plays.iter().map(Played::line).collect();
            if let Err(e) = std::fs::write(path, all) {
                log::warn!("cannot compact {}: {e}", path.display());
            }
        }
        h
    }

    /// Adds a play and appends it to `path`.
    pub fn record(&mut self, path: &Path, played: Played) {
        if let Err(e) = append(path, &played.line()) {
            log::warn!("cannot write {}: {e}", path.display());
        }
        self.push(played);
    }

    fn push(&mut self, played: Played) {
        while self.plays.len() >= self.limit {
            self.plays.pop_front();
        }
        self.plays.push_back(played);
    }

    /// Keeps `limit` plays from now on, dropping the oldest at once if there
    /// are more. The file shrinks the next time it is loaded.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.plays.len() > self.limit {
            self.plays.pop_front();
        }
    }

    /// Newest first.
    pub fn newest_first(&self) -> impl Iterator<Item = &Played> {
        self.plays.iter().rev()
    }

    pub fn len(&self) -> usize {
        self.plays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plays.is_empty()
    }
}

/// Seconds since the Unix epoch, now.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn append(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::OpenOptions::new().create(true).append(true).open(path)?.write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn played(at: u64, location: &str, title: &str) -> Played {
        Played { at, location: location.into(), title: title.into() }
    }

    fn file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("popamp-history-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("history.txt")
    }

    #[test]
    fn plays_survive_a_restart_newest_first() {
        let path = file("restart");
        let mut h = History::load(&path, DEFAULT_LIMIT);
        h.record(&path, played(100, "/music/a.flac", "A - One"));
        h.record(&path, played(200, "http://radio.example/live", "Radio"));
        let again = History::load(&path, DEFAULT_LIMIT);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        let titles: Vec<_> = again.newest_first().map(|p| (p.at, p.title.as_str())).collect();
        assert_eq!(titles, [(200, "Radio"), (100, "A - One")]);
    }

    #[test]
    fn odd_characters_keep_to_their_line() {
        let odd = played(5, "/music/tab\there\\x.mp3", "line\nbreak\r and \\t");
        let h = History::parse(&odd.line(), DEFAULT_LIMIT);
        assert_eq!(h.newest_first().next(), Some(&odd));
    }

    #[test]
    fn broken_lines_are_skipped() {
        let h = History::parse("garbage\n12\t\tno location\n7\t/a.mp3\tA\nx\t/b.mp3\tB\n8\t/c.mp3\n", DEFAULT_LIMIT);
        let locations: Vec<_> = h.newest_first().map(|p| p.location.as_str()).collect();
        assert_eq!(locations, ["/a.mp3"]);
    }

    #[test]
    fn only_the_newest_plays_are_kept_and_the_file_is_compacted() {
        let path = file("limit");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let limit = DEFAULT_LIMIT;
        let lines: String = (0..2 * limit as u64 + 1).map(|i| played(i, &format!("/{i}.mp3"), "").line()).collect();
        std::fs::write(&path, lines).unwrap();
        let h = History::load(&path, limit);
        let kept = std::fs::read_to_string(&path).unwrap().lines().count();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        assert_eq!((h.len(), kept), (limit, limit));
        assert_eq!(h.newest_first().next().map(|p| p.at), Some(2 * limit as u64));
    }

    #[test]
    fn the_limit_can_be_changed_and_trims_at_once() {
        let path = file("setlimit");
        let mut h = History::load(&path, 5);
        for i in 0..8 {
            h.record(&path, played(i, &format!("/{i}.mp3"), ""));
        }
        assert_eq!(h.len(), 5, "kept to the limit it was loaded with");
        h.set_limit(3);
        assert_eq!(h.newest_first().map(|p| p.at).collect::<Vec<_>>(), [7, 6, 5]);
        h.set_limit(100);
        h.record(&path, played(9, "/9.mp3", ""));
        assert_eq!(h.len(), 4, "a larger limit lets it grow again");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
