//! Cache module for Google Photos data.

use rusqlite::{params, Connection};
use rusqlite_migration::{M, Migrations};
use std::path::Path;
use std::error::Error;
use std::fmt;
use chrono::{DateTime, Utc, NaiveDateTime};

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
                 creation_time INTEGER NOT NULL,\n\
                 width INTEGER NOT NULL,\n\
                 height INTEGER NOT NULL,\n\
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
        M::up(
            "CREATE TABLE IF NOT EXISTS albums (\n\
                 id TEXT PRIMARY KEY,\n\
                 title TEXT,\n\
                 product_url TEXT,\n\
                 is_writeable INTEGER,\n\
                 media_items_count INTEGER,\n\
                 cover_photo_base_url TEXT,\n\
                 cover_photo_media_item_id TEXT\n\
             );\n\
             CREATE TABLE IF NOT EXISTS album_media_items (\n\
                 album_id TEXT NOT NULL,\n\
                 media_item_id TEXT NOT NULL,\n\
                 PRIMARY KEY(album_id, media_item_id),\n\
                 FOREIGN KEY(album_id) REFERENCES albums(id),\n\
                 FOREIGN KEY(media_item_id) REFERENCES media_items(id)\n\
             );\n\
             UPDATE schema_version SET version = 4;"
        ),
        M::up(
            "PRAGMA foreign_keys=OFF;\n\
             ALTER TABLE media_items RENAME TO media_items_old;\n\
             CREATE TABLE media_items (\n\
                 id TEXT PRIMARY KEY,\n\
                 description TEXT,\n\
                 product_url TEXT NOT NULL,\n\
                 base_url TEXT NOT NULL,\n\
                 mime_type TEXT NOT NULL,\n\
                 creation_time INTEGER NOT NULL,\n\
                 width INTEGER NOT NULL,\n\
                 height INTEGER NOT NULL,\n\
                 filename TEXT NOT NULL,\n\
                 is_favorite INTEGER NOT NULL DEFAULT 0\n\
             );\n\
             INSERT INTO media_items (id, description, product_url, base_url, mime_type, creation_time, width, height, filename, is_favorite)\n\
                 SELECT id, description, product_url, base_url, mime_type, CAST(strftime('%s', creation_time) AS INTEGER), CAST(width AS INTEGER), CAST(height AS INTEGER), filename, is_favorite FROM media_items_old;\n\
             DROP TABLE media_items_old;\n\
             DROP TABLE album_media_items;\n\
             CREATE TABLE album_media_items (\n\
                 album_id TEXT NOT NULL,\n\
                 media_item_id TEXT NOT NULL,\n\
                 PRIMARY KEY(album_id, media_item_id),\n\
                 FOREIGN KEY(album_id) REFERENCES albums(id),\n\
                 FOREIGN KEY(media_item_id) REFERENCES media_items(id)\n\
             );\n\
             PRAGMA foreign_keys=ON;\n\
             UPDATE schema_version SET version = 5;"
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
        let ts = DateTime::parse_from_rfc3339(&item.media_metadata.creation_time)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?
            .timestamp();
        let width: i64 = item
            .media_metadata
            .width
            .parse::<i64>()
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        let height: i64 = item
            .media_metadata
            .height
            .parse::<i64>()
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
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
                    ts,
                    width,
                    height,
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
            let ts: i64 = row.get(5).map_err(|e| CacheError::DatabaseError(e.to_string()))?;
            let dt = DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(ts, 0).ok_or_else(|| CacheError::DeserializationError("invalid timestamp".to_string()))?,
                Utc,
            );
            let item = api_client::MediaItem {
                id: row.get(0).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                description: row.get(1).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                product_url: row.get(2).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                base_url: row.get(3).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                mime_type: row.get(4).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                media_metadata: api_client::MediaMetadata {
                    creation_time: dt.to_rfc3339(),
                    width: row.get::<_, i64>(6).map_err(|e| CacheError::DatabaseError(e.to_string()))?.to_string(),
                    height: row.get::<_, i64>(7).map_err(|e| CacheError::DatabaseError(e.to_string()))?.to_string(),
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
                let ts: i64 = row.get(5)?;
                let dt = DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(rusqlite::Error::InvalidColumnType(5, "INTEGER".into(), rusqlite::types::Type::Integer))?,
                    Utc,
                );
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: dt.to_rfc3339(),
                        width: row.get::<_, i64>(6)?.to_string(),
                        height: row.get::<_, i64>(7)?.to_string(),
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
                let ts: i64 = row.get(5)?;
                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(rusqlite::Error::InvalidColumnType(5, "INTEGER".into(), rusqlite::types::Type::Integer))?,
                    Utc,
                );
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: dt.to_rfc3339(),
                        width: row.get::<_, i64>(6)?.to_string(),
                        height: row.get::<_, i64>(7)?.to_string(),
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
                let ts: i64 = row.get(5)?;
                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(rusqlite::Error::InvalidColumnType(5, "INTEGER".into(), rusqlite::types::Type::Integer))?,
                    Utc,
                );
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: dt.to_rfc3339(),
                        width: row.get::<_, i64>(6)?.to_string(),
                        height: row.get::<_, i64>(7)?.to_string(),
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

    /// Insert or update an album record.
    pub fn insert_album(&self, album: &api_client::Album) -> Result<(), CacheError> {
        let writable = album.is_writeable.map(|b| if b { 1 } else { 0 });
        let count: Option<i64> = match &album.media_items_count {
            Some(c) => c.parse().ok(),
            None => None,
        };
        self.conn
            .execute(
                "INSERT OR REPLACE INTO albums (
                    id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    album.id,
                    album.title,
                    album.product_url,
                    writable,
                    count,
                    album.cover_photo_base_url,
                    album.cover_photo_media_item_id,
                ],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert album: {}", e)))?;
        Ok(())
    }

    /// Link a media item to an album.
    pub fn add_media_item_to_album(&self, album_id: &str, media_item_id: &str) -> Result<(), CacheError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO album_media_items (album_id, media_item_id) VALUES (?1, ?2)",
                params![album_id, media_item_id],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to link media item to album: {}", e)))?;
        Ok(())
    }

    /// Retrieve all albums stored in the cache.
    pub fn get_all_albums(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let mut stmt = self.conn
            .prepare("SELECT id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id FROM albums")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map([], |row| {
                Ok(api_client::Album {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    product_url: row.get(2)?,
                    is_writeable: row.get::<_, Option<i64>>(3)?.map(|v| v != 0),
                    media_items_count: row.get::<_, Option<i64>>(4)?.map(|v| v.to_string()),
                    cover_photo_base_url: row.get(5)?,
                    cover_photo_media_item_id: row.get(6)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query albums: {}", e)))?;

        let mut albums = Vec::new();
        for album in iter {
            albums.push(album.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve album: {}", e)))?);
        }
        Ok(albums)
    }

    /// Retrieve media items associated with a specific album.
    pub fn get_media_items_by_album(&self, album_id: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, m.creation_time, m.width, m.height, m.filename
                 FROM media_items m INNER JOIN album_media_items a ON m.id = a.media_item_id
                 WHERE a.album_id = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![album_id], |row| {
                let ts: i64 = row.get(5)?;
                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(rusqlite::Error::InvalidColumnType(5, "INTEGER".into(), rusqlite::types::Type::Integer))?,
                    Utc,
                );
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: dt.to_rfc3339(),
                        width: row.get::<_, i64>(6)?.to_string(),
                        height: row.get::<_, i64>(7)?.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items by album: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item: {}", e)))?);
        }
        Ok(items)
    }

    /// Retrieve media items within the given creation time range.
    pub fn get_media_items_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT id, description, product_url, base_url, mime_type, creation_time, width, height, filename
                 FROM media_items WHERE creation_time BETWEEN ?1 AND ?2",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![start.timestamp(), end.timestamp()], |row| {
                let ts: i64 = row.get(5)?;
                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(ts, 0).ok_or(rusqlite::Error::InvalidColumnType(5, "INTEGER".into(), rusqlite::types::Type::Integer))?,
                    Utc,
                );
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: dt.to_rfc3339(),
                        width: row.get::<_, i64>(6)?.to_string(),
                        height: row.get::<_, i64>(7)?.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items by date: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve media item: {}", e)))?);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_client::{MediaItem, MediaMetadata, Album};
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
    fn test_album_and_date_queries() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        let item2 = create_test_media_item("id2");
        cache_manager.insert_media_item(&item1).unwrap();
        cache_manager.insert_media_item(&item2).unwrap();

        let album = api_client::Album {
            id: "alb1".to_string(),
            title: Some("Test".to_string()),
            product_url: None,
            is_writeable: Some(true),
            media_items_count: Some("2".to_string()),
            cover_photo_base_url: None,
            cover_photo_media_item_id: None,
        };
        cache_manager.insert_album(&album).unwrap();
        cache_manager.add_media_item_to_album(&album.id, &item1.id).unwrap();
        cache_manager.add_media_item_to_album(&album.id, &item2.id).unwrap();

        let albums = cache_manager.get_all_albums().unwrap();
        assert_eq!(albums.len(), 1);

        let by_album = cache_manager.get_media_items_by_album(&album.id).unwrap();
        assert_eq!(by_album.len(), 2);

        let start = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2023-01-02T00:00:00Z").unwrap().with_timezone(&Utc);
        let range_items = cache_manager.get_media_items_by_date_range(start, end).unwrap();
        assert_eq!(range_items.len(), 2);
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
        assert_eq!(version, 5);

        let mut stmt = conn.prepare("PRAGMA table_info(media_items)").unwrap();
        let cols: Vec<(String,String)> = stmt
            .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(cols.iter().any(|c| c.0 == "is_favorite"));
        let ct_type = cols.iter().find(|c| c.0 == "creation_time").unwrap().1.to_uppercase();
        assert_eq!(ct_type, "INTEGER");

        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='last_sync'").unwrap();
        let has_table: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        assert!(has_table.is_some());

        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='albums'").unwrap();
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
