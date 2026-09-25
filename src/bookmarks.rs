use crate::config::AppPaths;
use crate::location::Location;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BookmarkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[allow(dead_code)]
    #[error("Bookmark '{0}' not found")]
    NotFound(String),

    #[error("Bookmark name cannot be empty")]
    EmptyName,
}

/// Manages persistent named location profiles and bookmarks.
#[derive(Debug, Clone)]
pub struct BookmarkManager {
    path: PathBuf,
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BookmarkManager {
    /// Create a BookmarkManager using the default application bookmarks file.
    pub fn new() -> Self {
        Self {
            path: AppPaths::bookmarks_file(),
        }
    }

    /// Create a BookmarkManager with a custom path (useful for testing).
    #[allow(dead_code)]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Load all stored bookmarks.
    pub fn load(&self) -> Result<BTreeMap<String, Location>, BookmarkError> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let content = fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        let map = serde_json::from_str(&content)?;
        Ok(map)
    }

    /// Save a named bookmark. Overwrites if a bookmark with the same name exists.
    pub fn save(&self, name: &str, location: &Location) -> Result<(), BookmarkError> {
        let key = name.trim();
        if key.is_empty() {
            return Err(BookmarkError::EmptyName);
        }

        let mut bookmarks = self.load().unwrap_or_default();
        bookmarks.insert(key.to_string(), location.clone());

        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(&bookmarks)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Retrieve a bookmark by name (case-insensitive lookup).
    pub fn get(&self, name: &str) -> Result<Option<Location>, BookmarkError> {
        let key = name.trim().to_lowercase();
        let bookmarks = self.load()?;
        for (k, v) in bookmarks {
            if k.to_lowercase() == key {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    /// List all bookmarks sorted by name.
    pub fn list(&self) -> Result<Vec<(String, Location)>, BookmarkError> {
        let bookmarks = self.load()?;
        Ok(bookmarks.into_iter().collect())
    }

    /// Delete a bookmark by name (case-insensitive). Returns true if found and removed.
    pub fn delete(&self, name: &str) -> Result<bool, BookmarkError> {
        let key = name.trim().to_lowercase();
        let mut bookmarks = self.load()?;
        let mut removed = false;
        bookmarks.retain(|k, _| {
            if k.to_lowercase() == key {
                removed = true;
                false
            } else {
                true
            }
        });

        if removed {
            let json = serde_json::to_string_pretty(&bookmarks)?;
            fs::write(&self.path, json)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("locsim_bm_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let bm_path = temp_dir.join("test_bookmarks.json");
        let manager = BookmarkManager::with_path(&bm_path);

        // Initially empty
        assert!(manager.list().unwrap().is_empty());
        assert_eq!(manager.get("home").unwrap(), None);

        // Save a bookmark
        let loc_home = Location::new("Home Office", "123 Main St", 37.7749, -122.4194).unwrap();
        manager.save("home", &loc_home).expect("Save bookmark");

        // Save second bookmark
        let loc_work = Location::new("Work HQ", "456 Market St", 40.7128, -74.0060).unwrap();
        manager.save("work", &loc_work).expect("Save bookmark");

        // Case-insensitive retrieval
        let retrieved = manager.get("HOME").unwrap().expect("Should find home");
        assert_eq!(retrieved.name, "Home Office");
        assert_eq!(retrieved.latitude, 37.7749);

        // List
        let list = manager.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "home");
        assert_eq!(list[1].0, "work");

        // Delete
        assert!(manager.delete("Work").unwrap());
        assert_eq!(manager.list().unwrap().len(), 1);
        assert_eq!(manager.get("work").unwrap(), None);

        // Delete non-existent
        assert!(!manager.delete("nonexistent").unwrap());

        // Cleanup
        let _ = fs::remove_file(&bm_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_empty_bookmark_name_errors() {
        let manager = BookmarkManager::with_path(std::env::temp_dir().join("dummy.json"));
        let loc = Location::new("Test", "Address", 10.0, 20.0).unwrap();
        assert!(matches!(manager.save("   ", &loc), Err(BookmarkError::EmptyName)));
    }
}
