//! Cache module for Google Photos data.

use rusqlite::{Connection, params};
use std::path::Path;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CacheError {
    DatabaseError(String),
    SerializationError(String),
    DeserializationError(String),
    Other(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CacheError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
            CacheError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            CacheError::DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
            CacheError::Other(msg) => write!(f, "Other Error: {}", msg),
        }
    }
}

impl Error for CacheError {}

pub struct CacheManager {
    conn: Connection,
}

impl CacheManager {
    pub fn new(db_path: &Path) -> Result<Self, CacheError> {
        let conn = Connection::open(db_path)
            .map_err(|e| CacheError::DatabaseError(format!("Failed to open database: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS media_items (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                creation_time TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                product_url TEXT NOT NULL,
                base_url TEXT NOT NULL,
                data BLOB NOT NULL
            )",
            [],
        ).map_err(|e| CacheError::DatabaseError(format!("Failed to create table: {}", e)))?;

        Ok(CacheManager { conn })
    }

    pub fn insert_media_item(&self, item: &api_client::MediaItem) -> Result<(), CacheError> {
        let id = &item.id;
        let filename = &item.filename;
        let mime_type = &item.mime_type;
        let creation_time = &item.media_metadata.creation_time;
        let width = &item.media_metadata.width;
        let height = &item.media_metadata.height;
        let product_url = &item.product_url;
        let base_url = &item.base_url;

        let data = serde_json::to_vec(item)
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize media item: {}", e)))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO media_items (
                id, filename, mime_type, creation_time, width, height, product_url, base_url, data
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                filename,
                mime_type,
                creation_time,
                width,
                height,
                product_url,
                base_url,
                data
            ],
        ).map_err(|e| CacheError::DatabaseError(format!("Failed to insert media item: {}", e)))?;

        Ok(())
    }

    pub fn get_media_item(&self, id: &str) -> Result<Option<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn.prepare("SELECT data FROM media_items WHERE id = ?1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let mut rows = stmt.query(params![id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media item: {}", e)))?;

        if let Some(row) = rows.next().map_err(|e| CacheError::DatabaseError(format!("Failed to get row: {}", e)))? {
            let data: Vec<u8> = row.get(0)
                .map_err(|e| CacheError::DatabaseError(format!("Failed to get data from row: {}", e)))?;
            let item: api_client::MediaItem = serde_json::from_slice(&data)
                .map_err(|e| CacheError::DeserializationError(format!("Failed to deserialize media item: {}", e)))?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn.prepare("SELECT data FROM media_items")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let media_item_iter = stmt.query_map([], |row| {
            let data: Vec<u8> = row.get(0)?;
            let item: api_client::MediaItem = serde_json::from_slice(&data)
                .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?; // A bit of a hack, but it works for now
            Ok(item)
        }).map_err(|e| CacheError::DatabaseError(format!("Failed to query all media items: {}", e)))?;

        let mut items = Vec::new();
        for item_result in media_item_iter {
            items.push(item_result
                .map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e)))?);
        }
        Ok(items)
    }

    pub fn delete_media_item(&self, id: &str) -> Result<(), CacheError> {
        self.conn.execute("DELETE FROM media_items WHERE id = ?1", params![id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to delete media item: {}", e)))?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        self.conn.execute("DELETE FROM media_items", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear cache: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_client::{MediaItem, MediaMetadata};
    use tempfile::NamedTempFile;

    fn create_test_media_item(id: &str) -> MediaItem {
        // This test helper needs to be updated to match the new MediaItem structure
        // For now, we create a simplified version
        MediaItem {
            id: id.to_string(),
            description: None,
            product_url: format!("https://photos.google.com/lr/photo/{}", id),
            base_url: format!("https://lh3.googleusercontent.com/test/{}", id),
            mime_type: "image/jpeg".to_string(),
            media_metadata: MediaMetadata {
                creation_time: "2023-01-01T12:00:00Z".to_string(),
                width: "1920".to_string(),
                height: "1080".to_string(),
            },
            filename: format!("test_image_{}.jpg", id),
        }
    }

    #[test]
    fn test_cache_manager_new() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path);
        assert!(cache_manager.is_ok());
    }

    #[test]
    fn test_insert_and_get_media_item() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        cache_manager.insert_media_item(&item1).expect("Failed to insert item1");

        let retrieved_item = cache_manager.get_media_item("id1").expect("Failed to get item1");
        assert!(retrieved_item.is_some());
        assert_eq!(retrieved_item.unwrap().id, item1.id);

        let item2 = create_test_media_item("id2");
        cache_manager.insert_media_item(&item2).expect("Failed to insert item2");

        let retrieved_item_none = cache_manager.get_media_item("nonexistent_id").expect("Failed to get nonexistent item");
        assert!(retrieved_item_none.is_none());
    }

    #[test]
    fn test_get_all_media_items() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        let item2 = create_test_media_item("id2");

        cache_manager.insert_media_item(&item1).expect("Failed to insert item1");
        cache_manager.insert_media_item(&item2).expect("Failed to insert item2");

        let all_items = cache_manager.get_all_media_items().expect("Failed to get all items");
        assert_eq!(all_items.len(), 2);
        assert!(all_items.iter().any(|i| i.id == item1.id));
        assert!(all_items.iter().any(|i| i.id == item2.id));
    }

    #[test]
    fn test_delete_media_item() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        cache_manager.insert_media_item(&item1).expect("Failed to insert item1");

        cache_manager.delete_media_item("id1").expect("Failed to delete item1");
        let retrieved_item = cache_manager.get_media_item("id1").expect("Failed to get item1 after deletion");
        assert!(retrieved_item.is_none());
    }

    #[test]
    fn test_clear_cache() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        let item2 = create_test_media_item("id2");

        cache_manager.insert_media_item(&item1).expect("Failed to insert item1");
        cache_manager.insert_media_item(&item2).expect("Failed to insert item2");

        cache_manager.clear_cache().expect("Failed to clear cache");
        let all_items = cache_manager.get_all_media_items().expect("Failed to get all items after clear");
        assert!(all_items.is_empty());
    }
}