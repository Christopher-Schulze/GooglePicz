//! Cache module for Google Photos data.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};
use std::error::Error;
use std::fmt;
use std::path::Path;

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
        M::up(
            "CREATE TABLE IF NOT EXISTS albums (\n\
                 id TEXT PRIMARY KEY,\n\
                 title TEXT,\n\
                 product_url TEXT,\n\
                 is_writeable INTEGER,\n\
                 media_items_count TEXT,\n\
                 cover_photo_base_url TEXT,\n\
                 cover_photo_media_item_id TEXT\n\
             );\n\
             CREATE TABLE IF NOT EXISTS album_media_items (\n\
                 album_id TEXT NOT NULL,\n\
                 media_item_id TEXT NOT NULL,\n\
                 PRIMARY KEY (album_id, media_item_id),\n\
                 FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE,\n\
                 FOREIGN KEY(media_item_id) REFERENCES media_items(id) ON DELETE CASCADE\n\
             );\n\
             UPDATE schema_version SET version = 4;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS media_metadata (\n\
                 media_item_id TEXT PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,\n\
                 creation_time TEXT NOT NULL,\n\
                 width TEXT NOT NULL,\n\
                 height TEXT NOT NULL\n\
             );\n\
             INSERT OR IGNORE INTO media_metadata (media_item_id, creation_time, width, height)\n\
                 SELECT id, creation_time, width, height FROM media_items;\n\
             UPDATE schema_version SET version = 5;"
        ),
        M::up(
            "PRAGMA foreign_keys=off;\n\
             CREATE TABLE media_items_new (\n\
                 id TEXT PRIMARY KEY,\n\
                 description TEXT,\n\
                 product_url TEXT NOT NULL,\n\
                 base_url TEXT NOT NULL,\n\
                 mime_type TEXT NOT NULL,\n\
                 is_favorite INTEGER NOT NULL DEFAULT 0,\n\
                 filename TEXT NOT NULL\n\
             );\n\
             INSERT INTO media_items_new (id, description, product_url, base_url, mime_type, is_favorite, filename)\n\
                 SELECT id, description, product_url, base_url, mime_type, is_favorite, filename FROM media_items;\n\
             CREATE TABLE media_metadata_new (\n\
                 media_item_id TEXT PRIMARY KEY REFERENCES media_items_new(id) ON DELETE CASCADE,\n\
                 creation_time INTEGER NOT NULL,\n\
                 width INTEGER NOT NULL,\n\
                 height INTEGER NOT NULL\n\
             );\n\
             INSERT INTO media_metadata_new (media_item_id, creation_time, width, height)\n\
                 SELECT media_item_id, strftime('%s', creation_time), CAST(width AS INTEGER), CAST(height AS INTEGER) FROM media_metadata;\n\
             CREATE TABLE album_media_items_new (\n\
                 album_id TEXT NOT NULL,\n\
                 media_item_id TEXT NOT NULL,\n\
                 PRIMARY KEY (album_id, media_item_id),\n\
                 FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE,\n\
                 FOREIGN KEY(media_item_id) REFERENCES media_items_new(id) ON DELETE CASCADE\n\
             );\n\
             INSERT INTO album_media_items_new SELECT * FROM album_media_items;\n\
             DROP TABLE album_media_items;\n\
             DROP TABLE media_metadata;\n\
             DROP TABLE media_items;\n\
             ALTER TABLE media_items_new RENAME TO media_items;\n\
             ALTER TABLE media_metadata_new RENAME TO media_metadata;\n\
             ALTER TABLE album_media_items_new RENAME TO album_media_items;\n\
             PRAGMA foreign_keys=on;\n\
             UPDATE schema_version SET version = 6;"
        ),
    ]);
    migrations
        .to_latest(conn)
        .map_err(|e| CacheError::DatabaseError(format!("Failed to apply migrations: {}", e)))?;
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
        let creation_ts = DateTime::parse_from_rfc3339(&item.media_metadata.creation_time)
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
                    id, description, product_url, base_url, mime_type, filename
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item.id,
                    item.description,
                    item.product_url,
                    item.base_url,
                    item.mime_type,
                    item.filename
                ],
            )
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to insert media item: {}", e))
            })?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO media_metadata (
                    media_item_id, creation_time, width, height
                ) VALUES (?1, ?2, ?3, ?4)",
                params![item.id, creation_ts, width, height],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert metadata: {}", e)))?;

        Ok(())
    }

    pub fn get_media_item(&self, id: &str) -> Result<Option<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.id = ?1",
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
                id: row
                    .get(0)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                description: row
                    .get(1)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                product_url: row
                    .get(2)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                base_url: row
                    .get(3)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                mime_type: row
                    .get(4)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                media_metadata: api_client::MediaMetadata {
                    creation_time: {
                        let ts: i64 = row
                            .get(5)
                            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).ok_or_else(|| {
                                CacheError::DeserializationError("invalid timestamp".into())
                            })?,
                            Utc,
                        )
                        .to_rfc3339()
                    },
                    width: {
                        let w: i64 = row
                            .get(6)
                            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        w.to_string()
                    },
                    height: {
                        let h: i64 = row
                            .get(7)
                            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        h.to_string()
                    },
                },
                filename: row
                    .get(8)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?,
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
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let media_item_iter = stmt
            .query_map([], |row| {
                let ts: i64 = row.get(5)?;
                let w: i64 = row.get(6)?;
                let h: i64 = row.get(7)?;
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap(),
                            Utc,
                        )
                        .to_rfc3339(),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query all media items: {}", e))
            })?;

        let mut items = Vec::new();
        for item_result in media_item_iter {
            items.push(item_result.map_err(|e| {
                CacheError::DatabaseError(format!(
                    "Failed to retrieve media item from iterator: {}",
                    e
                ))
            })?);
        }
        Ok(items)
    }

    /// Retrieve all media items matching the given MIME type.
    pub fn get_media_items_by_mime_type(
        &self,
        mime: &str,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.mime_type = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![mime], |row| {
                let ts: i64 = row.get(5)?;
                let w: i64 = row.get(6)?;
                let h: i64 = row.get(7)?;
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap(),
                            Utc,
                        )
                        .to_rfc3339(),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query media items: {}", e))
            })?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!(
                    "Failed to retrieve media item from iterator: {}",
                    e
                ))
            })?);
        }
        Ok(items)
    }

    /// Retrieve media items where the filename contains the given pattern.
    pub fn get_media_items_by_filename(
        &self,
        pattern: &str,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let like_pattern = format!("%{}%", pattern);
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.filename LIKE ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![like_pattern], |row| {
                let ts: i64 = row.get(5)?;
                let w: i64 = row.get(6)?;
                let h: i64 = row.get(7)?;
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap(),
                            Utc,
                        )
                        .to_rfc3339(),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query media items: {}", e))
            })?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!(
                    "Failed to retrieve media item from iterator: {}",
                    e
                ))
            })?);
        }
        Ok(items)
    }

    pub fn insert_album(&self, album: &api_client::Album) -> Result<(), CacheError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO albums (
                    id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    album.id,
                    album.title,
                    album.product_url,
                    album.is_writeable.map(|b| if b { 1 } else { 0 }),
                    album.media_items_count,
                    album.cover_photo_base_url,
                    album.cover_photo_media_item_id
                ],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert album: {}", e)))?;

        Ok(())
    }

    /// Retrieve all albums from the cache.
    pub fn get_all_albums(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id FROM albums",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map([], |row| {
                Ok(api_client::Album {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    product_url: row.get(2)?,
                    is_writeable: row.get::<_, Option<i64>>(3)?.map(|v| v != 0),
                    media_items_count: row.get(4)?,
                    cover_photo_base_url: row.get(5)?,
                    cover_photo_media_item_id: row.get(6)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query albums: {}", e)))?;

        let mut albums = Vec::new();
        for album in iter {
            albums.push(album.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve album: {}", e))
            })?);
        }
        Ok(albums)
    }

    pub fn associate_media_item_with_album(
        &self,
        media_item_id: &str,
        album_id: &str,
    ) -> Result<(), CacheError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO album_media_items (album_id, media_item_id) VALUES (?1, ?2)",
                params![album_id, media_item_id],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to associate media item with album: {}", e)))?;
        Ok(())
    }

    pub fn get_media_items_by_album(
        &self,
        album_id: &str,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN album_media_items ami ON m.id = ami.media_item_id
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE ami.album_id = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![album_id], |row| {
                let ts: i64 = row.get(5)?;
                let w: i64 = row.get(6)?;
                let h: i64 = row.get(7)?;
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap(),
                            Utc,
                        )
                        .to_rfc3339(),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query media items by album: {}", e))
            })?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!(
                    "Failed to retrieve media item from iterator: {}",
                    e
                ))
            })?);
        }
        Ok(items)
    }

    pub fn get_media_items_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut stmt = self.conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE md.creation_time >= ?1 AND md.creation_time <= ?2",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![start.timestamp(), end.timestamp()], |row| {
                let ts: i64 = row.get(5)?;
                let w: i64 = row.get(6)?;
                let h: i64 = row.get(7)?;
                Ok(api_client::MediaItem {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    product_url: row.get(2)?,
                    base_url: row.get(3)?,
                    mime_type: row.get(4)?,
                    media_metadata: api_client::MediaMetadata {
                        creation_time: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp_opt(ts, 0).unwrap(),
                            Utc,
                        )
                        .to_rfc3339(),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query media items by date: {}", e))
            })?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!(
                    "Failed to retrieve media item from iterator: {}",
                    e
                ))
            })?);
        }
        Ok(items)
    }

    pub fn delete_media_item(&self, id: &str) -> Result<(), CacheError> {
        self.conn
            .execute("DELETE FROM media_items WHERE id = ?1", params![id])
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to delete media item: {}", e))
            })?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        self.conn
            .execute("DELETE FROM media_items", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear cache: {}", e)))?;
        Ok(())
    }

    pub fn get_last_sync(&self) -> Result<DateTime<Utc>, CacheError> {
        let mut stmt = self
            .conn
            .prepare("SELECT timestamp FROM last_sync WHERE id = 1")
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to prepare statement: {}", e))
            })?;
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
    use api_client::{Album, MediaItem, MediaMetadata};
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

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
        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");

        let retrieved_item = cache_manager
            .get_media_item("id1")
            .expect("Failed to get item1");
        assert!(retrieved_item.is_some());
        assert_eq!(retrieved_item.unwrap().id, item1.id);

        let item2 = create_test_media_item("id2");
        cache_manager
            .insert_media_item(&item2)
            .expect("Failed to insert item2");

        let retrieved_item_none = cache_manager
            .get_media_item("nonexistent_id")
            .expect("Failed to get nonexistent item");
        assert!(retrieved_item_none.is_none());
    }

    #[test]
    fn test_get_all_media_items() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        let item2 = create_test_media_item("id2");

        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");
        cache_manager
            .insert_media_item(&item2)
            .expect("Failed to insert item2");

        let all_items = cache_manager
            .get_all_media_items()
            .expect("Failed to get all items");
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

        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");
        cache_manager
            .insert_media_item(&item2)
            .expect("Failed to insert item2");

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
    fn test_query_by_album_and_date_range() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let album = Album {
            id: "a1".to_string(),
            title: Some("Test".to_string()),
            product_url: None,
            is_writeable: Some(true),
            media_items_count: None,
            cover_photo_base_url: None,
            cover_photo_media_item_id: None,
        };
        cache_manager
            .insert_album(&album)
            .expect("Failed to insert album");

        let item1 = create_test_media_item("id1");
        let mut item2 = create_test_media_item("id2");
        item2.media_metadata.creation_time = "2023-02-01T12:00:00Z".to_string();

        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");
        cache_manager
            .insert_media_item(&item2)
            .expect("Failed to insert item2");
        cache_manager
            .associate_media_item_with_album("id1", "a1")
            .expect("Failed to associate");

        let album_items = cache_manager
            .get_media_items_by_album("a1")
            .expect("Failed to query by album");
        assert_eq!(album_items.len(), 1);
        assert_eq!(album_items[0].id, item1.id);

        let start = DateTime::parse_from_rfc3339("2023-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2023-03-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let date_items = cache_manager
            .get_media_items_by_date_range(start, end)
            .expect("Failed to query by date");
        assert_eq!(date_items.len(), 1);
        assert_eq!(date_items[0].id, item2.id);
    }

    #[test]
    fn test_insert_and_get_all_albums() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let album = Album {
            id: "a1".to_string(),
            title: Some("Test".to_string()),
            product_url: None,
            is_writeable: Some(true),
            media_items_count: None,
            cover_photo_base_url: None,
            cover_photo_media_item_id: None,
        };
        cache_manager
            .insert_album(&album)
            .expect("Failed to insert album");

        let albums = cache_manager
            .get_all_albums()
            .expect("Failed to get albums");
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].id, album.id);
    }

    #[test]
    fn test_delete_media_item() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");

        cache_manager
            .delete_media_item("id1")
            .expect("Failed to delete item1");
        let retrieved_item = cache_manager
            .get_media_item("id1")
            .expect("Failed to get item1 after deletion");
        assert!(retrieved_item.is_none());
    }

    #[test]
    fn test_clear_cache() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let item1 = create_test_media_item("id1");
        let item2 = create_test_media_item("id2");

        cache_manager
            .insert_media_item(&item1)
            .expect("Failed to insert item1");
        cache_manager
            .insert_media_item(&item2)
            .expect("Failed to insert item2");

        cache_manager.clear_cache().expect("Failed to clear cache");
        let all_items = cache_manager
            .get_all_media_items()
            .expect("Failed to get all items after clear");
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
            )
            .unwrap();
            conn.execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
                .unwrap();
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])
                .unwrap();
            conn.pragma_update(None, "user_version", &1).unwrap();
        }

        let _cm = CacheManager::new(db_path).expect("Failed to open cache manager");

        let conn = Connection::open(db_path).unwrap();
        let version: i32 = conn
            .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 6);

        let mut stmt = conn.prepare("PRAGMA table_info(media_items)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(cols.contains(&"is_favorite".to_string()));

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='last_sync'")
            .unwrap();
        let has_table: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        assert!(has_table.is_some());

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='albums'")
            .unwrap();
        assert!(stmt
            .query_row([], |row| row.get::<_, String>(0))
            .ok()
            .is_some());

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='media_metadata'")
            .unwrap();
        assert!(stmt
            .query_row([], |row| row.get::<_, String>(0))
            .ok()
            .is_some());

        let mut stmt = conn.prepare("PRAGMA table_info(media_metadata)").unwrap();
        let cols: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let ct_type = cols
            .iter()
            .find(|(n, _)| n == "creation_time")
            .unwrap()
            .1
            .clone();
        assert_eq!(ct_type.to_uppercase(), "INTEGER");
    }

    #[test]
    fn test_apply_migrations_from_v3() {
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
                    filename TEXT NOT NULL,
                    is_favorite INTEGER NOT NULL DEFAULT 0
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "CREATE TABLE last_sync (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL)",
                [],
            )
            .unwrap();
            conn.execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
                .unwrap();
            conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])
                .unwrap();
            conn.pragma_update(None, "user_version", &3).unwrap();
        }

        let _cm = CacheManager::new(db_path).expect("Failed to open cache manager");

        let conn = Connection::open(db_path).unwrap();
        let version: i32 = conn
            .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 6);

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='albums'")
            .unwrap();
        let has_albums: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        assert!(has_albums.is_some());

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='media_metadata'")
            .unwrap();
        let has_meta: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        assert!(has_meta.is_some());

        let mut stmt = conn.prepare("PRAGMA table_info(media_metadata)").unwrap();
        let cols: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let ct_type = cols
            .iter()
            .find(|(n, _)| n == "creation_time")
            .unwrap()
            .1
            .clone();
        assert_eq!(ct_type.to_uppercase(), "INTEGER");
    }

    #[test]
    fn test_last_sync_functions() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();
        let cache_manager = CacheManager::new(db_path).expect("Failed to create cache manager");

        let ts = cache_manager
            .get_last_sync()
            .expect("Failed to get last sync");
        assert_eq!(ts, DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH));

        let now = Utc::now();
        cache_manager
            .update_last_sync(now)
            .expect("Failed to update last sync");

        let new_ts = cache_manager
            .get_last_sync()
            .expect("Failed to read updated last sync");
        assert!(new_ts >= now);
    }
}
