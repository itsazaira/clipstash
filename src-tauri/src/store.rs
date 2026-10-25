//! Clipboard history, kept in one JSON file in the app data folder.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// How many unpinned items to keep. Pinned items do not count towards this limit.
pub const HISTORY_SIZES: [usize; 4] = [10, 20, 50, 100];
pub const DEFAULT_HISTORY_SIZE: usize = 50;

/// Very large copies (whole files, logs) are skipped to keep the history file small.
const MAX_TEXT_BYTES: usize = 1_000_000;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub text: String,
    /// Unix time in milliseconds.
    pub copied_at: i64,
    pub pinned: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Data {
    /// Newest first.
    pub items: Vec<Item>,
    pub paused: bool,
    pub next_id: u64,
    pub first_run_done: bool,
    pub max_items: usize,
}

impl Default for Data {
    fn default() -> Self {
        Data { items: Vec::new(), paused: false, next_id: 0, first_run_done: false, max_items: DEFAULT_HISTORY_SIZE }
    }
}

/// What the popup renders: pinned items on top, then newest first.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub items: Vec<Item>,
    pub paused: bool,
    pub max_items: usize,
}

pub struct Store {
    path: PathBuf,
    pub data: Data,
    /// Last clipboard text seen by the watcher, so the same copy is not saved twice.
    pub last_seen: String,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let mut data = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Data>(&s).ok())
            .unwrap_or_default();
        if !HISTORY_SIZES.contains(&data.max_items) {
            data.max_items = DEFAULT_HISTORY_SIZE;
        }
        Store { path, data, last_seen: String::new() }
    }

    /// Writes to a temp file first, then renames, so a crash never leaves a half written file.
    pub fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let Ok(json) = serde_json::to_string(&self.data) else { return };
        let tmp = self.path.with_extension("json.tmp");
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(&tmp, &self.path);
        }
    }

    /// Adds a new copy to the top. Returns false when nothing changed.
    pub fn add(&mut self, text: String) -> bool {
        if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
            return false;
        }
        if self.data.items.first().is_some_and(|top| top.text == text) {
            return false;
        }
        self.data.next_id += 1;
        self.data.items.insert(
            0,
            Item { id: self.data.next_id, text, copied_at: now_ms(), pinned: false },
        );
        self.prune();
        self.save();
        true
    }

    /// Drops the oldest unpinned items beyond the limit.
    fn prune(&mut self) {
        let max = self.data.max_items;
        let mut unpinned = 0;
        self.data.items.retain(|item| {
            if item.pinned {
                return true;
            }
            unpinned += 1;
            unpinned <= max
        });
    }

    /// Moves an item back to the top (it was just used) and returns its text.
    pub fn touch(&mut self, id: u64) -> Option<String> {
        let pos = self.data.items.iter().position(|i| i.id == id)?;
        let mut item = self.data.items.remove(pos);
        item.copied_at = now_ms();
        let text = item.text.clone();
        self.data.items.insert(0, item);
        self.save();
        Some(text)
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(item) = self.data.items.iter_mut().find(|i| i.id == id) {
            item.pinned = !item.pinned;
        }
        self.prune();
        self.save();
    }

    /// Removes every unpinned item. Pinned items stay until unpinned.
    pub fn clear(&mut self) {
        self.data.items.retain(|i| i.pinned);
        self.save();
    }

    /// Changes how many copies are kept. Going lower drops the oldest unpinned items now.
    pub fn set_max_items(&mut self, max: usize) -> bool {
        if !HISTORY_SIZES.contains(&max) {
            return false;
        }
        self.data.max_items = max;
        self.prune();
        self.save();
        true
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.data.paused = paused;
        self.save();
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut items: Vec<Item> = self.data.items.iter().filter(|i| i.pinned).cloned().collect();
        items.extend(self.data.items.iter().filter(|i| !i.pinned).cloned());
        Snapshot { items, paused: self.data.paused, max_items: self.data.max_items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!("clipstash-test-{}.json", now_ms()));
        Store { path, data: Data::default(), last_seen: String::new() }
    }

    #[test]
    fn skips_adjacent_duplicates_and_blank_text() {
        let mut s = store();
        assert!(s.add("a".into()));
        assert!(!s.add("a".into()));
        assert!(!s.add("   ".into()));
        assert!(s.add("b".into()));
        assert!(s.add("a".into()));
        assert_eq!(s.data.items.len(), 3);
    }

    #[test]
    fn keeps_fifty_unpinned_plus_pinned() {
        let mut s = store();
        s.add("pinned".into());
        let id = s.data.items[0].id;
        s.toggle_pin(id);
        for n in 0..60 {
            s.add(format!("item {n}"));
        }
        assert_eq!(s.data.items.iter().filter(|i| !i.pinned).count(), DEFAULT_HISTORY_SIZE);
        assert!(s.set_max_items(10));
        assert_eq!(s.data.items.iter().filter(|i| !i.pinned).count(), 10);
        assert!(!s.set_max_items(7));
        assert_eq!(s.snapshot().items[0].text, "pinned");
        s.clear();
        assert_eq!(s.data.items.len(), 1);
    }
}
