//! Cache module for Google Photos data.

use chrono::{DateTime, Utc, TimeZone};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use rusqlite_migration::{Migrations, M};
use thiserror::Error;
use std::path::Path;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Database Error: {0}")]
    DatabaseError(String),
    #[error("Serialization Error: {0}")]
    SerializationError(String),
    #[error("Deserialization Error: {0}")]
    DeserializationError(String),
    #[error("Other Error: {0}")]
    Other(String),
}

#[derive(Clone)]
pub struct CacheManager {
    conn: Arc<Mutex<Connection>>,
}

fn apply_migrations(conn: &mut Connection) -> Result<(), CacheError> {
    let migrations = Migrations::new(vec![
        M::up(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);\
             INSERT INTO schema_version (version) VALUES (1);\
             CREATE TABLE IF NOT EXISTS media_items (\
                 id TEXT PRIMARY KEY,\
                 description TEXT,\
                 product_url TEXT NOT NULL,\
                 base_url TEXT NOT NULL,\
                 mime_type TEXT NOT NULL,\
                 creation_time TEXT NOT NULL,\
                 width TEXT NOT NULL,\
                 height TEXT NOT NULL,\
                 filename TEXT NOT NULL\
             );"
        ),
        M::up(
            "ALTER TABLE media_items ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;\
             UPDATE schema_version SET version = 2;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS last_sync (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL);\
             INSERT OR IGNORE INTO last_sync (id, timestamp) VALUES (1, '1970-01-01T00:00:00Z');\
             UPDATE schema_version SET version = 3;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS albums (\
                 id TEXT PRIMARY KEY,\
                 title TEXT,\
                 product_url TEXT,\
                 is_writeable INTEGER,\
                 media_items_count TEXT,\
                 cover_photo_base_url TEXT,\
                 cover_photo_media_item_id TEXT\
             );\
             CREATE TABLE IF NOT EXISTS album_media_items (\
                 album_id TEXT NOT NULL,\
                 media_item_id TEXT NOT NULL,\
                 PRIMARY KEY (album_id, media_item_id),\
                 FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE,\
                 FOREIGN KEY(media_item_id) REFERENCES media_items(id) ON DELETE CASCADE\
             );\
             UPDATE schema_version SET version = 4;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS media_metadata (\
                 media_item_id TEXT PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,\
                 creation_time TEXT NOT NULL,\
                 width TEXT NOT NULL,\
                 height TEXT NOT NULL\
             );\
             INSERT OR IGNORE INTO media_metadata (media_item_id, creation_time, width, height)\
                 SELECT id, creation_time, width, height FROM media_items;\
             UPDATE schema_version SET version = 5;"
        ),
        M::up(
            "PRAGMA foreign_keys=off;\
             CREATE TABLE media_items_new (\
                 id TEXT PRIMARY KEY,\
                 description TEXT,\
                 product_url TEXT NOT NULL,\
                 base_url TEXT NOT NULL,\
                 mime_type TEXT NOT NULL,\
                 is_favorite INTEGER NOT NULL DEFAULT 0,\
                 filename TEXT NOT NULL\
             );\
             INSERT INTO media_items_new (id, description, product_url, base_url, mime_type, is_favorite, filename)\
                 SELECT id, description, product_url, base_url, mime_type, is_favorite, filename FROM media_items;\
             CREATE TABLE media_metadata_new (\
                 media_item_id TEXT PRIMARY KEY REFERENCES media_items_new(id) ON DELETE CASCADE,\
                 creation_time INTEGER NOT NULL,\
                 width INTEGER NOT NULL,\
                 height INTEGER NOT NULL\
             );\
             INSERT INTO media_metadata_new (media_item_id, creation_time, width, height)\
                 SELECT media_item_id, strftime('%s', creation_time), CAST(width AS INTEGER), CAST(height AS INTEGER) FROM media_metadata;\
             CREATE TABLE album_media_items_new (\
                 album_id TEXT NOT NULL,\
                 media_item_id TEXT NOT NULL,\
                 PRIMARY KEY (album_id, media_item_id),\
                 FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE,\
                 FOREIGN KEY(media_item_id) REFERENCES media_items_new(id) ON DELETE CASCADE\
             );\
             INSERT INTO album_media_items_new SELECT * FROM album_media_items;\
             DROP TABLE album_media_items;\
             DROP TABLE media_metadata;\
             DROP TABLE media_items;\
             ALTER TABLE media_items_new RENAME TO media_items;\
             ALTER TABLE media_metadata_new RENAME TO media_metadata;\
            ALTER TABLE album_media_items_new RENAME TO album_media_items;\
            PRAGMA foreign_keys=on;\
            UPDATE schema_version SET version = 6;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_media_metadata_creation_time ON media_metadata (creation_time);\
             CREATE INDEX IF NOT EXISTS idx_media_items_mime_type ON media_items (mime_type);\
             UPDATE schema_version SET version = 7;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_album_media_items_album_id ON album_media_items (album_id);\
             UPDATE schema_version SET version = 8;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_album_media_items_media_item_id ON album_media_items (media_item_id);\
             UPDATE schema_version SET version = 9;"
        ),
        M::up(
            "PRAGMA foreign_keys=off;\
             CREATE TABLE albums_new (\
                 id TEXT PRIMARY KEY,\
                 title TEXT,\
                 product_url TEXT,\
                 is_writeable INTEGER,\
                 media_items_count INTEGER,\
                 cover_photo_base_url TEXT,\
                 cover_photo_media_item_id TEXT,\
                 FOREIGN KEY(cover_photo_media_item_id) REFERENCES media_items(id) ON DELETE SET NULL\
             );\
             INSERT INTO albums_new (id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id)\
                 SELECT id, title, product_url, is_writeable, CAST(media_items_count AS INTEGER), cover_photo_base_url, cover_photo_media_item_id FROM albums;\
             DROP TABLE albums;\
             ALTER TABLE albums_new RENAME TO albums;\
             PRAGMA foreign_keys=on;\
             UPDATE schema_version SET version = 10;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_media_items_is_favorite ON media_items (is_favorite);\
             UPDATE schema_version SET version = 11;"
        ),
    ]);
    migrations
        .to_latest(conn)
        .map_err(|e| CacheError::DatabaseError(format!("Failed to apply migrations: {}", e)))?;
    Ok(())
}

impl CacheManager {
    pub fn lock_conn(&self) -> Result<std::sync::MutexGuard<Connection>, CacheError> {
        self.conn
            .lock()
            .map_err(|_| CacheError::Other("Poisoned lock".into()))
    }
    fn ts_to_rfc3339(ts: i64) -> String {
        DateTime::<Utc>::from_timestamp(ts, 0)
            .unwrap_or_else(|| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
            .to_rfc3339()
    }
    pub fn new(db_path: &Path) -> Result<Self, CacheError> {
        let mut conn = Connection::open(db_path)
            .map_err(|e| CacheError::DatabaseError(format!("Failed to open database: {}", e)))?;
        apply_migrations(&mut conn)?;

        Ok(CacheManager { conn: Arc::new(Mutex::new(conn)) })
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

        let conn = self.lock_conn()?;
        conn
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

        conn
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
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                id: row.get(0).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                description: row.get(1).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                product_url: row.get(2).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                base_url: row.get(3).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                mime_type: row.get(4).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                media_metadata: api_client::MediaMetadata {
                    creation_time: {
                        let ts: i64 = row.get(5).map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        Self::ts_to_rfc3339(ts)
                    },
                    width: {
                        let w: i64 = row.get(6).map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        w.to_string()
                    },
                    height: {
                        let h: i64 = row.get(7).map_err(|e| CacheError::DatabaseError(e.to_string()))?;
                        h.to_string()
                    },
                },
                filename: row.get(8).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
            };
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start = std::time::Instant::now();
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                        creation_time: Self::ts_to_rfc3339(ts),
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
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("cache_load_time_ms" = %start.elapsed().as_millis(), "items" = items.len());
        Ok(items)
    }

    pub fn get_media_items_by_mime_type(&self, mime: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn get_media_items_by_filename(&self, pattern: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let like_pattern = format!("%{}%", pattern);
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn get_favorite_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.is_favorite = 1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn get_media_items_by_favorite(&self, fav: bool) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let mut conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.is_favorite = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![if fav { 1 } else { 0 }], |row| {
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn insert_album(&self, album: &api_client::Album) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
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

    pub fn delete_album(&self, album_id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute("DELETE FROM albums WHERE id = ?1", params![album_id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to delete album: {}", e)))?;
        Ok(())
    }

    pub fn rename_album(&self, album_id: &str, new_title: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute(
                "UPDATE albums SET title = ?1 WHERE id = ?2",
                params![new_title, album_id],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to rename album: {}", e)))?;
        Ok(())
    }

    pub fn get_all_albums(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
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

    pub fn associate_media_item_with_album(&self, media_item_id: &str, album_id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute(
                "INSERT OR REPLACE INTO album_media_items (album_id, media_item_id) VALUES (?1, ?2)",
                params![album_id, media_item_id],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to associate media item with album: {}", e)))?;
        Ok(())
    }

    pub fn get_media_items_by_album(&self, album_id: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items by album: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn get_media_items_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
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
                        creation_time: Self::ts_to_rfc3339(ts),
                        width: w.to_string(),
                        height: h.to_string(),
                    },
                    filename: row.get(8)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items by date: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        Ok(items)
    }

    pub fn delete_media_item(&self, id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute("DELETE FROM media_items WHERE id = ?1", params![id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to delete media item: {}", e)))?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute("DELETE FROM album_media_items", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear album_media_items: {}", e)))?;
        conn
            .execute("DELETE FROM albums", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear albums: {}", e)))?;
        conn
            .execute("DELETE FROM media_metadata", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear media_metadata: {}", e)))?;
        conn
            .execute("DELETE FROM media_items", [])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear cache: {}", e)))?;
        conn
            .execute(
                "UPDATE last_sync SET timestamp = '1970-01-01T00:00:00Z' WHERE id = 1",
                [],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to reset last_sync: {}", e)))?;
        Ok(())
    }

    pub fn get_last_sync(&self) -> Result<DateTime<Utc>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare("SELECT timestamp FROM last_sync WHERE id = 1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        let ts: String = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query last sync: {}", e)))?;
        DateTime::parse_from_rfc3339(&ts)
            .map_err(|e| CacheError::DeserializationError(e.to_string()))
            .map(|dt| Utc.from_utc_datetime(&dt.naive_utc()))
    }

    pub fn update_last_sync(&self, ts: DateTime<Utc>) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        conn
            .execute(
                "UPDATE last_sync SET timestamp = ?1 WHERE id = 1",
                params![ts.to_rfc3339()],
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to update last sync: {}", e)))?;
        Ok(())
    }

    pub async fn insert_media_item_async(&self, item: api_client::MediaItem) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_media_item(&item))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_all_media_items_async(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_all_media_items())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_last_sync_async(&self) -> Result<DateTime<Utc>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_last_sync())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn update_last_sync_async(&self, ts: DateTime<Utc>) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.update_last_sync(ts))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn insert_album_async(&self, album: api_client::Album) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_album(&album))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn associate_media_item_with_album_async(
        &self,
        media_item_id: String,
        album_id: String,
    ) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.associate_media_item_with_album(&media_item_id, &album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn rename_album_async(&self, album_id: String, new_title: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.rename_album(&album_id, &new_title))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn delete_album_async(&self, album_id: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.delete_album(&album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn delete_media_item_async(&self, id: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.delete_media_item(&id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_all_albums_async(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_all_albums())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_media_items_by_album_async(&self, album_id: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_album(&album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_favorite_media_items_async(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_favorite_media_items())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_media_items_by_favorite_async(&self, fav: bool) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_favorite(fav))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_media_items_by_date_range_async(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_date_range(start, end))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }
}

// Die Unit-Tests sind wie in deinem Input (ausgelassen für Zeichenlimit), aber alles vollständig!
// Bei Bedarf schick ich dir die Tests als eigenen Block – sag nur Bescheid.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_media_item(id: &str) -> api_client::MediaItem {
        api_client::MediaItem {
            id: id.to_string(),
            description: Some("desc".into()),
            product_url: "http://example.com".into(),
            base_url: "http://example.com/base".into(),
            mime_type: "image/jpeg".into(),
            media_metadata: api_client::MediaMetadata {
                creation_time: "2023-01-01T00:00:00Z".into(),
                width: "1".into(),
                height: "1".into(),
            },
            filename: format!("{}.jpg", id),
        }
    }

    fn sample_album(id: &str) -> api_client::Album {
        api_client::Album {
            id: id.to_string(),
            title: Some("Album".into()),
            product_url: None,
            is_writeable: None,
            media_items_count: None,
            cover_photo_base_url: None,
            cover_photo_media_item_id: None,
        }
    }

    #[test]
    fn test_clear_cache_empties_all_tables() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let cache = CacheManager::new(tmp.path()).expect("create cache manager");

        let item = sample_media_item("1");
        cache.insert_media_item(&item).expect("insert media");
        let album = sample_album("a1");
        cache.insert_album(&album).expect("insert album");
        cache
            .associate_media_item_with_album(&item.id, &album.id)
            .expect("associate");
        cache.update_last_sync(Utc::now()).expect("update last sync");

        assert_eq!(cache.get_all_media_items().unwrap().len(), 1);
        assert_eq!(cache.get_all_albums().unwrap().len(), 1);
        assert_eq!(
            cache
                .get_media_items_by_album(&album.id)
                .unwrap()
                .len(),
            1
        );

        cache.clear_cache().expect("clear cache");

        let conn = Connection::open(tmp.path()).expect("open connection");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_metadata", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM albums", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM album_media_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let ts: String = conn
            .query_row("SELECT timestamp FROM last_sync WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ts, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_cache_manager_new_invalid_path() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let result = CacheManager::new(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_media_item_invalid_metadata() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let cache = CacheManager::new(tmp.path()).expect("create cache manager");
        let mut item = sample_media_item("1");
        item.media_metadata.width = "not_a_number".into();
        let result = cache.insert_media_item(&item);
        assert!(matches!(result, Err(CacheError::SerializationError(_))));
    }

    #[test]
    fn test_get_favorite_media_items() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let cache = CacheManager::new(tmp.path()).expect("create cache manager");

        let fav = sample_media_item("fav");
        cache.insert_media_item(&fav).expect("insert fav");
        {
            let conn = cache.conn.lock().unwrap();
            conn.execute(
                "UPDATE media_items SET is_favorite = 1 WHERE id = ?1",
                params![fav.id],
            )
            .unwrap();
        }
        let not_fav = sample_media_item("n1");
        cache.insert_media_item(&not_fav).expect("insert");

        let items = cache.get_favorite_media_items().expect("query");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, fav.id);
    }

    #[test]
    fn test_get_media_items_by_favorite() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let cache = CacheManager::new(tmp.path()).expect("create cache manager");

        let fav = sample_media_item("fav");
        cache.insert_media_item(&fav).expect("insert fav");
        {
            let conn = cache.conn.lock().unwrap();
            conn.execute(
                "UPDATE media_items SET is_favorite = 1 WHERE id = ?1",
                params![fav.id],
            )
            .unwrap();
        }

        let not_fav = sample_media_item("n1");
        cache.insert_media_item(&not_fav).expect("insert");

        let fav_items = cache.get_media_items_by_favorite(true).expect("query");
        assert_eq!(fav_items.len(), 1);
        assert_eq!(fav_items[0].id, fav.id);

        let not_items = cache.get_media_items_by_favorite(false).expect("query");
        assert_eq!(not_items.len(), 1);
        assert_eq!(not_items[0].id, not_fav.id);
    }
}

