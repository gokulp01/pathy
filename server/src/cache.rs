use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug)]
struct CacheEntry {
    dir: PathBuf,
    items: Vec<DirEntryInfo>,
    timestamp: Instant,
}

#[derive(Debug)]
pub struct DirCache {
    ttl: Duration,
    max_entries: usize,
    entries: VecDeque<CacheEntry>,
}

impl DirCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries,
            entries: VecDeque::new(),
        }
    }

    pub fn get(&mut self, dir: &Path) -> Option<Vec<DirEntryInfo>> {
        let now = Instant::now();
        if let Some(pos) = self.entries.iter().position(|entry| entry.dir == dir) {
            let entry = self.entries.remove(pos)?;
            if now.duration_since(entry.timestamp) <= self.ttl {
                let items = entry.items.clone();
                self.entries.push_front(entry);
                return Some(items);
            }
        }
        None
    }

    pub fn insert(&mut self, dir: &Path, items: Vec<DirEntryInfo>) {
        if let Some(pos) = self.entries.iter().position(|entry| entry.dir == dir) {
            self.entries.remove(pos);
        }
        self.entries.push_front(CacheEntry {
            dir: dir.to_path_buf(),
            items,
            timestamp: Instant::now(),
        });
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
    }

    pub fn update_limits(&mut self, ttl: Duration, max_entries: usize) {
        self.ttl = ttl;
        self.max_entries = max_entries;
        while self.entries.len() > self.max_entries {
            self.entries.pop_back();
        }
    }
}
