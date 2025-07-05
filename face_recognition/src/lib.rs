//! Face recognition module for GooglePicz.
//!
//! The current implementation only provides placeholder functions.
//! Real face detection will be integrated later.

use api_client::MediaItem;
#[cfg(feature = "cache")]
use cache::CacheManager;
use thiserror::Error;

/// Details about a detected face.
#[derive(Debug, Clone)]
pub struct Face {
    /// Optional name of the person.
    pub name: Option<String>,
}

#[derive(Debug, Error)]
pub enum FaceRecognitionError {
    #[error("Cache Error: {0}")]
    CacheError(String),
    #[error("Other Error: {0}")]
    Other(String),
}

/// Main struct providing face detection capabilities.
#[derive(Default)]
pub struct FaceRecognizer;

impl FaceRecognizer {
    /// Create a new face recognizer instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect faces in the given media item.
    ///
    /// Currently returns an empty list.
    pub fn detect_faces(&self, _item: &MediaItem) -> Result<Vec<Face>, FaceRecognitionError> {
        // TODO: integrate real face detection
        Ok(Vec::new())
    }

    /// Associate detected faces with a `MediaItem` in the cache.
    #[cfg(feature = "cache")]
    pub fn assign_to_cache(
        &self,
        _cache: &CacheManager,
        _item: &MediaItem,
        _faces: &[Face],
    ) -> Result<(), FaceRecognitionError> {
        // TODO: store faces in cache
        Ok(())
    }

    /// Prepare face data for display in the UI.
    #[cfg(feature = "ui")]
    pub fn prepare_ui(&self, _faces: &[Face]) {
        // TODO: forward faces to the UI layer
    }
}
