#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
//! Cache module for Google Photos data.

use chrono::{DateTime, Utc, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use rusqlite_migration::{Migrations, M};
use thiserror::Error;
use std::path::Path;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceData {
    pub bbox: [i32; 4],
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceExport {
    pub media_item_id: String,
    pub faces: Vec<FaceData>,
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
        M::up(
            "ALTER TABLE media_metadata ADD COLUMN camera_make TEXT;\
             ALTER TABLE media_metadata ADD COLUMN camera_model TEXT;\
             ALTER TABLE media_metadata ADD COLUMN fps REAL;\
             ALTER TABLE media_metadata ADD COLUMN status TEXT;\
             UPDATE schema_version SET version = 12;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_media_metadata_camera_model ON media_metadata (camera_model);\
             CREATE INDEX IF NOT EXISTS idx_media_items_filename ON media_items (filename);\
             CREATE INDEX IF NOT EXISTS idx_media_items_description ON media_items (description);\
             UPDATE schema_version SET version = 13;"
        ),
        M::up(
            "CREATE TABLE IF NOT EXISTS faces (\
                 media_item_id TEXT PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,\
                 faces_json TEXT NOT NULL\
             );\
             UPDATE schema_version SET version = 14;"
        ),
        M::up(
            "CREATE INDEX IF NOT EXISTS idx_media_metadata_camera_make ON media_metadata (camera_make);\
             UPDATE schema_version SET version = 15;"
        ),
        M::up(
            "CREATE VIRTUAL TABLE IF NOT EXISTS media_items_fts USING fts5(media_item_id UNINDEXED, filename, description);\
             INSERT INTO media_items_fts (media_item_id, filename, description) SELECT id, filename, coalesce(description, '') FROM media_items;\
             CREATE TRIGGER IF NOT EXISTS media_items_ai AFTER INSERT ON media_items BEGIN\
                 INSERT INTO media_items_fts (media_item_id, filename, description) VALUES (new.id, new.filename, coalesce(new.description, ''));\
             END;\
             CREATE TRIGGER IF NOT EXISTS media_items_ad AFTER DELETE ON media_items BEGIN\
                 DELETE FROM media_items_fts WHERE media_item_id = old.id;\
             END;\
             CREATE TRIGGER IF NOT EXISTS media_items_au AFTER UPDATE OF filename, description ON media_items BEGIN\
                 UPDATE media_items_fts SET filename = new.filename, description = coalesce(new.description, '') WHERE media_item_id = old.id;\
             END;\
             UPDATE schema_version SET version = 16;"
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
    #[cfg_attr(feature = "trace-spans", tracing::instrument)]
    pub fn new(db_path: &Path) -> Result<Self, CacheError> {
        let mut conn = Connection::open(db_path)
            .map_err(|e| CacheError::DatabaseError(format!("Failed to open database: {}", e)))?;
        apply_migrations(&mut conn)?;

        Ok(CacheManager { conn: Arc::new(Mutex::new(conn)) })
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, item)))]
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
        let mut item_stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO media_items (
                    id, description, product_url, base_url, mime_type, filename
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        item_stmt
            .execute(params![
                item.id,
                item.description,
                item.product_url,
                item.base_url,
                item.mime_type,
                item.filename
            ])
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to insert media item: {}", e))
            })?;

        let mut meta_stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO media_metadata (
                    media_item_id, creation_time, width, height, camera_make, camera_model, fps, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        meta_stmt
            .execute(params![
                item.id,
                creation_ts,
                width,
                height,
                item.media_metadata.video.as_ref().and_then(|v| v.camera_make.clone()),
                item.media_metadata.video.as_ref().and_then(|v| v.camera_model.clone()),
                item.media_metadata.video.as_ref().and_then(|v| v.fps),
                item.media_metadata.video.as_ref().and_then(|v| v.status.clone()),
            ])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert metadata: {}", e)))?;

        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, items)))]
    pub fn insert_media_items_batch(&self, items: &[api_client::MediaItem]) -> Result<(), CacheError> {
        let mut conn = self.lock_conn()?;
        let tx = conn
            .transaction()
            .map_err(|e| CacheError::DatabaseError(format!("Failed to start transaction: {}", e)))?;

        let mut item_stmt = tx
            .prepare_cached(
                "INSERT OR REPLACE INTO media_items (
                    id, description, product_url, base_url, mime_type, filename
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let mut meta_stmt = tx
            .prepare_cached(
                "INSERT OR REPLACE INTO media_metadata (
                    media_item_id, creation_time, width, height, camera_make, camera_model, fps, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        for item in items {
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

            item_stmt
                .execute(params![
                    item.id,
                    item.description,
                    item.product_url,
                    item.base_url,
                    item.mime_type,
                    item.filename
                ])
                .map_err(|e| CacheError::DatabaseError(format!("Failed to insert media item: {}", e)))?;

            meta_stmt
                .execute(params![
                    item.id,
                    creation_ts,
                    width,
                    height,
                    item.media_metadata.video.as_ref().and_then(|v| v.camera_make.clone()),
                    item.media_metadata.video.as_ref().and_then(|v| v.camera_model.clone()),
                    item.media_metadata.video.as_ref().and_then(|v| v.fps),
                    item.media_metadata.video.as_ref().and_then(|v| v.status.clone()),
                ])
                .map_err(|e| CacheError::DatabaseError(format!("Failed to insert metadata: {}", e)))?;
        }

        drop(item_stmt);
        drop(meta_stmt);
        tx.commit()
            .map_err(|e| CacheError::DatabaseError(format!("Failed to commit transaction: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_item(&self, id: &str) -> Result<Option<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                    video: Some(api_client::VideoMetadata {
                        camera_make: row.get(8).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                        camera_model: row.get(9).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                        fps: row.get(10).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                        status: row.get(11).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
                    }),
                },
                filename: row.get(12).map_err(|e| CacheError::DatabaseError(e.to_string()))?,
            };
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_all_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start_time = std::time::Instant::now();
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
                })
            })
            .map_err(|e| {
                CacheError::DatabaseError(format!("Failed to query all media items: {}", e))
            })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_items", [], |row| row.get(0))
            .map_err(|e| CacheError::DatabaseError(format!("Failed to count items: {}", e)))?;
        let mut items = Vec::with_capacity(count as usize);
        for item_result in media_item_iter {
            items.push(item_result.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("cache_load_time_ms" = %start_time.elapsed().as_millis(), "items" = items.len());
        Ok(items)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_mime_type(&self, mime: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    /// Retrieve media items filtered by optional camera model, date range and favorite flag.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn query_media_items(
        &self,
        camera_model: Option<&str>,
        camera_make: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        favorite: Option<bool>,
        mime_type: Option<&str>,
        text: Option<&str>,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start_time = std::time::Instant::now();
        let conn = self.lock_conn()?;
        let sql = concat!(
            "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename ",
            "FROM media_items m ",
            "JOIN media_metadata md ON m.id = md.media_item_id ",
            "WHERE (?1 IS NULL OR md.camera_model = ?1) ",
            "AND (?2 IS NULL OR md.camera_make = ?2) ",
            "AND (?3 IS NULL OR md.creation_time >= ?3) ",
            "AND (?4 IS NULL OR md.creation_time <= ?4) ",
            "AND (?5 IS NULL OR m.is_favorite = ?5) ",
            "AND (?6 IS NULL OR m.mime_type = ?6) ",
            "AND (?7 IS NULL OR m.filename LIKE ?7 OR m.description LIKE ?7)"
        );
        let mut stmt = conn
            .prepare_cached(sql)
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let fav_val: Option<i64> = favorite.map(|f| if f { 1 } else { 0 });
        let like_pattern = text.map(|t| format!("%{}%", t));

        let iter = stmt
            .query_map(
                params![
                    camera_model,
                    camera_make,
                    start.map(|s| s.timestamp()),
                    end.map(|e| e.timestamp()),
                    fav_val,
                    mime_type,
                    like_pattern.as_deref()
                ],
                |row| {
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
                            video: Some(api_client::VideoMetadata {
                                camera_make: row.get(8)?,
                                camera_model: row.get(9)?,
                                fps: row.get(10)?,
                                status: row.get(11)?,
                            }),
                        },
                        filename: row.get(12)?,
                    })
                },
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("query_time_ms" = %start_time.elapsed().as_millis(), "query" = "general", "count" = items.len());
        Ok(items)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_camera_model(&self, model: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start_time = std::time::Instant::now();
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE md.camera_model = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![model], |row| {
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("query_time_ms" = %start_time.elapsed().as_millis(), "query" = "camera_model", "count" = items.len());
        Ok(items)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_camera_make(&self, make: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start_time = std::time::Instant::now();
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE md.camera_make = ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![make], |row| {
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("query_time_ms" = %start_time.elapsed().as_millis(), "query" = "camera_make", "count" = items.len());
        Ok(items)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_filename(&self, pattern: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let start_time = std::time::Instant::now();
        let like_pattern = format!("%{}%", pattern);
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
                })
            })
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query media items: {}", e)))?;

        let mut items = Vec::new();
        for item in iter {
            items.push(item.map_err(|e| {
                CacheError::DatabaseError(format!("Failed to retrieve media item from iterator: {}", e))
            })?);
        }
        tracing::info!("query_time_ms" = %start_time.elapsed().as_millis(), "query" = "filename", "count" = items.len());
        Ok(items)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_description(&self, pattern: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let like_pattern = format!("%{}%", pattern);
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
                 FROM media_items m
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE m.description LIKE ?1",
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_text(&self, pattern: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
                 FROM media_items_fts f
                 JOIN media_items m ON m.id = f.media_item_id
                 JOIN media_metadata md ON m.id = md.media_item_id
                 WHERE f MATCH ?1",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let iter = stmt
            .query_map(params![pattern], |row| {
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_favorite_media_items(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_favorite(&self, fav: bool) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, album)))]
    pub fn insert_album(&self, album: &api_client::Album) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO albums (
                    id, title, product_url, is_writeable, media_items_count, cover_photo_base_url, cover_photo_media_item_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![
            album.id,
            album.title,
            album.product_url,
            album.is_writeable.map(|b| if b { 1 } else { 0 }),
            album.media_items_count,
            album.cover_photo_base_url,
            album.cover_photo_media_item_id
        ])
        .map_err(|e| CacheError::DatabaseError(format!("Failed to insert album: {}", e)))?;

        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn delete_album(&self, album_id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM albums WHERE id = ?1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![album_id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to delete album: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn rename_album(&self, album_id: &str, new_title: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("UPDATE albums SET title = ?1 WHERE id = ?2")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![new_title, album_id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to rename album: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_all_albums(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn associate_media_item_with_album(&self, media_item_id: &str, album_id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO album_media_items (album_id, media_item_id) VALUES (?1, ?2)",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![album_id, media_item_id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to associate media item with album: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn remove_media_item_from_album(&self, media_item_id: &str, album_id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "DELETE FROM album_media_items WHERE album_id = ?1 AND media_item_id = ?2",
            )
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![album_id, media_item_id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to remove media item from album: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_album(&self, album_id: &str) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_media_items_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT m.id, m.description, m.product_url, m.base_url, m.mime_type, md.creation_time, md.width, md.height, md.camera_make, md.camera_model, md.fps, md.status, m.filename
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
                        video: Some(api_client::VideoMetadata {
                            camera_make: row.get(8)?,
                            camera_model: row.get(9)?,
                            fps: row.get(10)?,
                            status: row.get(11)?,
                        }),
                    },
                    filename: row.get(12)?,
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn delete_media_item(&self, id: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM media_items WHERE id = ?1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to delete media item: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn set_favorite(&self, id: &str, fav: bool) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("UPDATE media_items SET is_favorite = ?1 WHERE id = ?2")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![if fav { 1 } else { 0 }, id])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to update favorite: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn clear_cache(&self) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM album_media_items")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute([])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear album_media_items: {}", e)))?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM albums")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute([])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear albums: {}", e)))?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM media_metadata")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute([])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear media_metadata: {}", e)))?;
        let mut stmt = conn
            .prepare_cached("DELETE FROM media_items")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute([])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to clear cache: {}", e)))?;
        let mut stmt = conn
            .prepare_cached("UPDATE last_sync SET timestamp = '1970-01-01T00:00:00Z' WHERE id = 1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute([])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to reset last_sync: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_last_sync(&self) -> Result<DateTime<Utc>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("SELECT timestamp FROM last_sync WHERE id = 1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        let ts: String = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query last sync: {}", e)))?;
        DateTime::parse_from_rfc3339(&ts)
            .map_err(|e| CacheError::DeserializationError(e.to_string()))
            .map(|dt| Utc.from_utc_datetime(&dt.naive_utc()))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn update_last_sync(&self, ts: DateTime<Utc>) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("UPDATE last_sync SET timestamp = ?1 WHERE id = 1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![ts.to_rfc3339()])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to update last sync: {}", e)))?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn export_media_items<P: AsRef<Path>>(&self, path: P) -> Result<(), CacheError> {
        let items = self.get_all_media_items()?;
        let file = std::fs::File::create(path.as_ref())
            .map_err(|e| CacheError::Other(format!("Failed to create export file: {}", e)))?;
        serde_json::to_writer(file, &items)
            .map_err(|e| CacheError::SerializationError(e.to_string()))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn export_albums<P: AsRef<Path>>(&self, path: P) -> Result<(), CacheError> {
        let albums = self.get_all_albums()?;
        let file = std::fs::File::create(path.as_ref())
            .map_err(|e| CacheError::Other(format!("Failed to create export file: {}", e)))?;
        serde_json::to_writer(file, &albums)
            .map_err(|e| CacheError::SerializationError(e.to_string()))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn export_faces<P: AsRef<Path>>(&self, path: P) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("SELECT media_item_id, faces_json FROM faces")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        let iter = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query faces: {}", e)))?;
        let mut data = Vec::new();
        for entry in iter {
            let (id, json) = entry.map_err(|e| CacheError::DatabaseError(format!("Failed to retrieve faces: {}", e)))?;
            let faces: Vec<FaceData> = serde_json::from_str(&json)
                .map_err(|e| CacheError::DeserializationError(e.to_string()))?;
            data.push(FaceExport { media_item_id: id, faces });
        }
        let file = std::fs::File::create(path.as_ref())
            .map_err(|e| CacheError::Other(format!("Failed to create export file: {}", e)))?;
        serde_json::to_writer(file, &data)
            .map_err(|e| CacheError::SerializationError(e.to_string()))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn import_media_items<P: AsRef<Path>>(&self, path: P) -> Result<(), CacheError> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| CacheError::Other(format!("Failed to open import file: {}", e)))?;
        let items: Vec<api_client::MediaItem> = serde_json::from_reader(file)
            .map_err(|e| CacheError::DeserializationError(e.to_string()))?;
        for item in &items {
            self.insert_media_item(item)?;
        }
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn import_faces<P: AsRef<Path>>(&self, path: P) -> Result<(), CacheError> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| CacheError::Other(format!("Failed to open import file: {}", e)))?;
        let entries: Vec<FaceExport> = serde_json::from_reader(file)
            .map_err(|e| CacheError::DeserializationError(e.to_string()))?;
        for entry in &entries {
            let json = serde_json::to_string(&entry.faces)
                .map_err(|e| CacheError::SerializationError(e.to_string()))?;
            self.insert_faces(&entry.media_item_id, &json)?;
        }
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn insert_faces(&self, media_item_id: &str, faces_json: &str) -> Result<(), CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR REPLACE INTO faces (media_item_id, faces_json) VALUES (?1, ?2)")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        stmt.execute(params![media_item_id, faces_json])
            .map_err(|e| CacheError::DatabaseError(format!("Failed to insert faces: {}", e)))?;
        Ok(())
    }

    #[cfg(feature = "face-recognition")]
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, faces_json)))]
    pub async fn insert_faces_async(
        &self,
        media_item_id: String,
        faces_json: String,
    ) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_faces(&media_item_id, &faces_json))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub fn get_faces(&self, media_item_id: &str) -> Result<Option<Vec<FaceData>>, CacheError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare_cached("SELECT faces_json FROM faces WHERE media_item_id = ?1")
            .map_err(|e| CacheError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        let faces_json: Option<String> = stmt
            .query_row(params![media_item_id], |row| row.get(0))
            .optional()
            .map_err(|e| CacheError::DatabaseError(format!("Failed to query faces: {}", e)))?;
        if let Some(json) = faces_json {
            let data: Vec<FaceData> = serde_json::from_str(&json)
                .map_err(|e| CacheError::DeserializationError(e.to_string()))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, item)))]
    pub async fn insert_media_item_async(&self, item: api_client::MediaItem) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_media_item(&item))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, items)))]
    pub async fn insert_media_items_batch_async(&self, items: Vec<api_client::MediaItem>) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_media_items_batch(&items))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_all_media_items_async(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_all_media_items())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_last_sync_async(&self) -> Result<DateTime<Utc>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_last_sync())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn update_last_sync_async(&self, ts: DateTime<Utc>) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.update_last_sync(ts))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn export_media_items_async<P>(&self, path: P) -> Result<(), CacheError>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.export_media_items(path))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn export_albums_async<P>(&self, path: P) -> Result<(), CacheError>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.export_albums(path))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn export_faces_async<P>(&self, path: P) -> Result<(), CacheError>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.export_faces(path))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn import_media_items_async<P>(&self, path: P) -> Result<(), CacheError>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.import_media_items(path))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn import_faces_async<P>(&self, path: P) -> Result<(), CacheError>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.import_faces(path))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, album)))]
    pub async fn insert_album_async(&self, album: api_client::Album) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_album(&album))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn remove_media_item_from_album_async(
        &self,
        media_item_id: String,
        album_id: String,
    ) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.remove_media_item_from_album(&media_item_id, &album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn rename_album_async(&self, album_id: String, new_title: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.rename_album(&album_id, &new_title))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn delete_album_async(&self, album_id: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.delete_album(&album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn delete_media_item_async(&self, id: String) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.delete_media_item(&id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn set_favorite_async(&self, id: String, fav: bool) -> Result<(), CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.set_favorite(&id, fav))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_all_albums_async(&self) -> Result<Vec<api_client::Album>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_all_albums())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_album_async(&self, album_id: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_album(&album_id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_favorite_media_items_async(&self) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_favorite_media_items())
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_favorite_async(&self, fav: bool) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_favorite(fav))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_date_range_async(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_date_range(start, end))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_media_items_by_description_async(&self, pattern: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_description(&pattern))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    pub async fn get_media_items_by_text_async(&self, pattern: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_text(&pattern))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_item_async(&self, id: String) -> Result<Option<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_item(&id))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_mime_type_async(&self, mime: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_mime_type(&mime))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_camera_model_async(&self, model: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_camera_model(&model))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_media_items_by_filename_async(&self, pattern: String) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.get_media_items_by_filename(&pattern))
            .await
            .map_err(|e| CacheError::Other(e.to_string()))?
    }

  
  
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn query_media_items_async(
        &self,
        camera_model: Option<String>,
        camera_make: Option<String>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        favorite: Option<bool>,
        mime_type: Option<String>,
        text: Option<String>,
    ) -> Result<Vec<api_client::MediaItem>, CacheError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.query_media_items(
                camera_model.as_deref(),
                camera_make.as_deref(),
                start,
                end,
                favorite,
                mime_type.as_deref(),
                text.as_deref(),
            )
        })
        .await
        .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg(feature = "face-recognition")]
    pub async fn get_faces_for_media_item(&self, id: &str) -> Result<Vec<face_recognition::Face>, CacheError> {
        let this = self.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            match this.get_faces(&id)? {
                Some(list) => Ok(list
                    .into_iter()
                    .map(|f| face_recognition::Face {
                        bbox: f.bbox,
                        name: f.name,
                        rect: (
                            f.bbox[0] as u32,
                            f.bbox[1] as u32,
                            f.bbox[2] as u32,
                            f.bbox[3] as u32,
                        ),
                    })
                    .collect()),
                None => Ok(Vec::new()),
            }
        })
        .await
        .map_err(|e| CacheError::Other(e.to_string()))?
    }

    #[cfg(feature = "face-recognition")]
    pub async fn update_face_name(&self, id: &str, idx: usize, name: &str) -> Result<(), CacheError> {
        let this = self.clone();
        let id = id.to_string();
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            let mut faces = this
                .get_faces(&id)?
                .unwrap_or_default();
            if idx < faces.len() {
                faces[idx].name = Some(name);
                let json = serde_json::to_string(&faces)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                this.insert_faces(&id, &json)?;
            }
            Ok(())
        })
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
                video: None,
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

    #[test]
    fn test_query_media_items_combined() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let cache = CacheManager::new(tmp.path()).expect("create cache manager");

        let mut item1 = sample_media_item("1");
        item1.media_metadata.creation_time = "2023-01-02T00:00:00Z".into();
        item1.media_metadata.video = Some(api_client::VideoMetadata {
            camera_make: Some("Canon".into()),
            camera_model: Some("EOS".into()),
            fps: None,
            status: None,
        });
        cache.insert_media_item(&item1).expect("insert1");
        {
            let conn = cache.conn.lock().unwrap();
            conn.execute(
                "UPDATE media_items SET is_favorite = 1 WHERE id = ?1",
                params![item1.id],
            )
            .unwrap();
        }

        let mut item2 = sample_media_item("2");
        item2.media_metadata.creation_time = "2023-02-01T00:00:00Z".into();
        item2.media_metadata.video = Some(api_client::VideoMetadata {
            camera_make: Some("Nikon".into()),
            camera_model: Some("D5".into()),
            fps: None,
            status: None,
        });
        cache.insert_media_item(&item2).expect("insert2");

        let start = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2023, 1, 31, 23, 59, 59).unwrap();
        let items = cache
            .query_media_items(
                Some("EOS"),
                None,
                Some(start),
                Some(end),
                Some(true),
                None,
                None,
            )
            .expect("query");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, item1.id);
    }
}

