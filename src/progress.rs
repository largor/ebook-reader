use anyhow::Result;
use dirs::data_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookProgress {
    pub chapter_index: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ProgressStore {
    books: HashMap<String, BookProgress>,
}

pub struct ProgressManager {
    store_path: PathBuf,
    store: ProgressStore,
}

impl ProgressManager {
    pub fn new() -> Self {
        let store_path = data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ebook-reader")
            .join("progress.json");

        let store = if store_path.exists() {
            fs::read_to_string(&store_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            ProgressStore::default()
        };

        Self { store_path, store }
    }

    pub fn get<P: AsRef<Path>>(&self, book_path: P) -> Option<BookProgress> {
        let key = book_path.as_ref().to_string_lossy().to_string();
        self.store.books.get(&key).cloned()
    }

    pub fn save<P: AsRef<Path>>(&mut self, book_path: P, progress: BookProgress) -> Result<()> {
        let key = book_path.as_ref().to_string_lossy().to_string();
        self.store.books.insert(key, progress);
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.store_path, json)?;
        Ok(())
    }
}
