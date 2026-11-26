//! Activity cache with 24-hour TTL.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::error::{AppError, Result};
use crate::models::Activity;

/// Cache TTL: 24 hours.
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Cached activity data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityCache {
    /// When the cache was last updated.
    pub updated_at: u64,
    /// Cached activities.
    pub activities: Vec<Activity>,
}

impl ActivityCache {
    /// Create a new cache with the given activities.
    pub fn new(activities: Vec<Activity>) -> Self {
        let updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            updated_at,
            activities,
        }
    }

    /// Check if the cache is still valid.
    ///
    /// An empty cache is never valid: Redmine always has activities, and older
    /// versions wrote empty caches during `--dry-run`, which would otherwise
    /// make real writes fail with "Unknown activity" until the TTL expired.
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        !self.activities.is_empty() && now - self.updated_at < CACHE_TTL.as_secs()
    }

    /// Get cache age as human-readable string.
    #[allow(dead_code)]
    pub fn age_string(&self) -> String {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_secs = now.saturating_sub(self.updated_at);

        if age_secs < 60 {
            format!("{}s ago", age_secs)
        } else if age_secs < 3600 {
            format!("{}m ago", age_secs / 60)
        } else {
            format!("{}h ago", age_secs / 3600)
        }
    }

    /// Find an activity by name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Option<&Activity> {
        let name_lower = name.to_lowercase();
        self.activities
            .iter()
            .find(|a| a.name.to_lowercase() == name_lower)
    }

    /// Find an activity by ID.
    pub fn find_by_id(&self, id: u32) -> Option<&Activity> {
        self.activities.iter().find(|a| a.id == id)
    }

    /// Resolve an activity by name or ID string.
    pub fn resolve(&self, name_or_id: &str) -> Option<&Activity> {
        // Try parsing as ID first
        if let Ok(id) = name_or_id.parse::<u32>() {
            if let Some(activity) = self.find_by_id(id) {
                return Some(activity);
            }
        }
        // Fall back to name lookup
        self.find_by_name(name_or_id)
    }

    /// Load cache from file.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let cache: Self = serde_json::from_str(&content)?;
        Ok(Some(cache))
    }

    /// Save cache to file.
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Resolve activity name/ID to activity ID, using cache.
pub fn resolve_activity(cache: &ActivityCache, name_or_id: &str) -> Result<u32> {
    cache.resolve(name_or_id).map(|a| a.id).ok_or_else(|| {
        AppError::validation_with_hint(
            format!("Unknown activity: '{}'", name_or_id),
            "Use `rdm time activities list` to see available activities.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_activities() -> Vec<Activity> {
        vec![
            Activity {
                id: 1,
                name: "Development".to_string(),
                is_default: Some(true),
            },
            Activity {
                id: 2,
                name: "Design".to_string(),
                is_default: None,
            },
            Activity {
                id: 3,
                name: "Testing".to_string(),
                is_default: None,
            },
        ]
    }

    #[test]
    fn test_cache_validity() {
        let cache = ActivityCache::new(test_activities());
        assert!(cache.is_valid());
    }

    #[test]
    fn test_find_by_name() {
        let cache = ActivityCache::new(test_activities());
        let activity = cache.find_by_name("development").unwrap();
        assert_eq!(activity.id, 1);
    }

    #[test]
    fn test_find_by_id() {
        let cache = ActivityCache::new(test_activities());
        let activity = cache.find_by_id(2).unwrap();
        assert_eq!(activity.name, "Design");
    }

    #[test]
    fn test_resolve_by_id() {
        let cache = ActivityCache::new(test_activities());
        let activity = cache.resolve("1").unwrap();
        assert_eq!(activity.name, "Development");
    }

    #[test]
    fn test_resolve_by_name() {
        let cache = ActivityCache::new(test_activities());
        let activity = cache.resolve("Testing").unwrap();
        assert_eq!(activity.id, 3);
    }

    #[test]
    fn test_cache_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activities.json");

        let cache = ActivityCache::new(test_activities());
        cache.save(&path).unwrap();

        let loaded = ActivityCache::load(&path).unwrap().unwrap();
        assert_eq!(loaded.activities.len(), 3);
        assert!(loaded.is_valid());
    }

    #[test]
    fn test_empty_cache_is_not_valid() {
        let cache = ActivityCache::new(vec![]);
        assert!(!cache.is_valid());
    }
}
