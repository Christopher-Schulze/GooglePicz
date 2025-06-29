//! Cache module for Google Photos data.

use rusqlite::{params, Connection};
use rusqlite_migration::{M, Migrations};
use std::path::Path;
use std::error::Error;
use std::fmt;
use chrono::{DateTime, Utc};

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

fn apply_migrations(conn: &mut Connection) -> Result<(), CacheError> {
    let migrations = Migrations::new(vec![
        M::up(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);\n\
             INSERT INTO schema_version (version) VALUES (1);\n\
             CREATE TABLE IF NOT EXISTS media_items (\n\
                 id TEXT PRIMARY KEY,\n\
                 description TEXT,\n\
                 product_url TEXT NOT NULL,\n\
                 base_url TEXT NOT NULL,\n\
                 mime_type TEXT NOT NULL,\n\
                 creation_time TEXT NOT NULL,\n\
                 width TEXT NOT NULL,\n\
                 height TEXT NOT NULL,\n\
                 filename TEXT NOT NULL\n\
             );"
        ),
        M::up(
            "ALTER TABLE media_items ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;\n\
             UPDATE schema_version SET version = 2;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS last_sync (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL);\n\
             INSERT OR IGNORE INTO last_sync (id, timestamp) VALUES (1, '1970-01-01T00:00:00Z');\n\
             UPDATE schema_version SET version = 3;"
        ),
    ]);
    migrations
        .to_latest(conn)
        .map_err(|e| CacheError::DatabaseError(format!("Failed to apply migrations: {}", e)))?
        ;
    Ok(())
}

impl CacheManager {
    pub fn new(db_path: &Path) -> Result<Self, CacheError> {
        let mut conn = Connection::open(db_path)
            .map_err(|e| CacheError::DatabaseError(format!("Failed to open database: {}", e)))?;
        apply_migrations(&mut conn)?;

        Ok(CacheManager { conn })
    }

    pub fn insert_media_item(&self, item: &api_client::MediaItem) -> Result<(), CacheError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO media_items (
                    id, description, product_url, base_url, mime_type, creation_time, width, height, filename
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    item.id,
                    item.description,
                    item.product_url,
                    item.base_url,
                    item.mime_type,
                    item.media_metadata.creation_time,
                    item.media_metadata.width,
                    item.media_metadata.height,
                    item.filename
                ],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert media item: {}", e)))?;

        Ok(())
    }

    pub fn get_media_item(&self, id: &str) -> Result<Option<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT id, description, product_url, base_url, mime_type, creation_time, width, height, filename FROM media_items WHERE id = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let mut rows = stmt
            .query(params![id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media item: {}", e)))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| CacheError::DatabaseError(format!("Failed to get row: {}", e)))?
        {
            let item = api_client::MediaItem {
                id: row.get(0).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                description: row.get(1).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                product_url: row.get(2).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                base_url: row.get(3).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                mime_type: row.get(4).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                media_metadata: api_client::MediaMetadata {
                    creation_time: row.get(5).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                    width: row.get(6).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                    height: row.get(7).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                },
                filename: row.get(8).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
            };
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, description, product_url, base_url, mime_type, creation_time, width, height, filename FROM media_items",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let media_item_iter = stmt
            .query_map([], |row| {
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: row.get(5)?,
                        width: row.get(6)?,
                        height: row.get(7)?,
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query all media items: {}", e)))?;

        let mut items = Vec::new();
        for item_result in media_item_iter {
            items.push(
                item_result
                    .map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e)))?,
            );
        }
        Ok(items)
    }

    /// Retrieve all media items matching the given MIME type.
    pub fn get_media_items_by_mime_type(&self, mime: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT id, description, product_url, base_url, mime_type, creation_time, width, height, filename FROM media_items WHERE mime_type = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![mime], |row| {
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: row.get(5)?,
                        width: row.get(6)?,
                        height: row.get(7)?,
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e)))?);
        }
        Ok(items)
    }

    /// Retrieve media items where the filename contains the given pattern.
    pub fn get_media_items_by_filename(&self, pattern: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let like_pattern = format!("%{}%", pattern);
        let mut stmt = self.conn
            .prepare(
                "SELECT id, description, product_url, base_url, mime_type, creation_time, width, height, filename FROM media_items WHERE filename LIKE ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![like_pattern], |row| {
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: row.get(5)?,
                        width: row.get(6)?,
                        height: row.get(7)?,
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e)))?);
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

    pub fn get_last_sync(&self) -> Result<DateTime<Utc>, CacheError> {
        let mut stmt = self.conn
            .prepare("SELECT timestamp FROM last_sync WHERE id = 1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        let ts: String = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query last sync: {}", e)))?;
        DateTime::parse_from_rfc3339(&ts)
            .map_err(|e| CacheError::DeserializationError(e.to_string()))
            .map(|dt| dt.with_timezone(&Utc))
    }

    pub fn update_last_sync(&self, ts: DateTime<Utc>) -> Result<(), CacheError> {
        self.conn
            .execute(
                "UPDATE last_sync SET timestamp = ?1 WHERE id = 1",
                params![ts.to_rfc3339()],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to update last sync: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_client::{MediaItem, MediaMetadata};
    use rusqlite::Connection;
    use tempfile::NamedTempFile;
    use chrono::{DateTime, Utc};

    fn create_test_media_item(id: &str) -> MediaItem {
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
    fn test_query_by_mime_type_and_filename() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let mut item1 = create_test_media_item("id1");
        item1.mime_type = "image/png".to_string();
        item1.filename = "holiday_photo.png".to_string();
        let mut item2 = create_test_media_item("id2");
        item2.mime_type = "image/jpeg".to_string();
        item2.filename = "family.jpg".to_string();

        cache_manager.insert_media_item(&item1).expect("Failed to insert item1");
        cache_manager.insert_media_item(&item2).expect("Failed to insert item2");

        let png_items = cache_manager
            .get_media_items_by_mime_type("image/png")
            .expect("Failed to query by mime type");
        assert_eq!(png_items.len(), 1);
        assert_eq!(png_items[0].id, item1.id);

        let filename_items = cache_manager
            .get_media_items_by_filename("family")
            .expect("Failed to query by filename");
        assert_eq!(filename_items.len(), 1);
        assert_eq!(filename_items[0].id, item2.id);
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

    #[test]
    fn test_apply_migrations_from_old_version() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();

        {
            let conn = Connection::open(db_path).unwrap();
            conn.execute(
                "CREATE TABLE media_items (
                    id TEXT PRIMARY KEY,
                    description TEXT,
                    product_url TEXT NOT NULL,
                    base_url TEXT NOT NULL,
                    mime_type TEXT NOT NULL,
                    creation_time TEXT NOT NULL,
                    width TEXT NOT NULL,
                    height TEXT NOT NULL,
                    filename TEXT NOT NULL
                )",
                [],
            ).unwrap();
            conn.execute(
                "CREATE TABLE schema_version (version INTEGER NOT NULL)",
                [],
            ).unwrap();
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", []).unwrap();
            conn.pragma_update(None, "user_version", &1).unwrap();
        }

        let _cm = CacheManager::new(db_path).expect("Failed to open cache manager");

        let conn = Connection::open(db_path).unwrap();
        let version: i32 = conn.query_row("SELECT version FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);

        let mut stmt = conn.prepare("PRAGMA table_info(media_items)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(cols.contains(&"is_favorite".to_string()));

        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='last_sync'").unwrap();
        let has_table: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        assert!(has_table.is_some());
    }

    #[test]
    fn test_last_sync_functions() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let ts = cache_manager.get_last_sync().expect("Failed to get last sync");
        assert_eq!(ts, DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH));

        let now = Utc::now();
        cache_manager.update_last_sync(now).expect("Failed to update last sync");

        let new_ts = cache_manager.get_last_sync().expect("Failed to read updated last sync");
        assert!(new_ts >= now);
    }
}
