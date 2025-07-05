//! Face recognition module for GooglePicz.
//!
//! The current implementation only provides placeholder functions.
//! Real face detection will be integrated later.

use api_client::MediaItem;
#[cfg(feature = "cache")]
use cache::CacheManager;
use opencv::{core, imgcodecs, imgproc, objdetect, prelude::*};
use reqwest::blocking as reqwest_blocking;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Details about a detected face.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    /// Bounding box of the face (x, y, width, height).
    pub bbox: [i32; 4],
    /// Optional name of the person.
    pub name: Option<String>,
    pub rect: (u32, u32, u32, u32),
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
    #[allow(clippy::too_many_lines)]
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, item)))]
    pub fn detect_faces(&self, item: &MediaItem) -> Result<Vec<Face>, FaceRecognitionError> {
        let bytes = if item.base_url.starts_with("file://") {
            let path = item.base_url.trim_start_matches("file://");
            std::fs::read(path).map_err(|e| FaceRecognitionError::Other(e.to_string()))?
        } else {
            let url = format!("{}=d", item.base_url);
            let resp = reqwest_blocking::get(&url)
                .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;
            resp.bytes()
                .map_err(|e| FaceRecognitionError::Other(e.to_string()))?
                .to_vec()
        };

        let data = core::Vector::from_slice(&bytes);
        let img = imgcodecs::imdecode(&data, imgcodecs::IMREAD_COLOR)
            .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;
        let mut gray = Mat::default();
        imgproc::cvt_color(&img, &mut gray, imgproc::COLOR_BGR2GRAY, 0)
            .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;

        let cascade_path = "/usr/share/opencv4/haarcascades/haarcascade_frontalface_default.xml";
        let mut classifier = objdetect::CascadeClassifier::new(cascade_path)
            .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;
        let mut rects = core::Vector::<core::Rect>::new();
        classifier
            .detect_multi_scale(
                &gray,
                &mut rects,
                1.1,
                3,
                0,
                core::Size::new(30, 30),
                core::Size::new(0, 0),
            )
            .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;

        let faces = rects
            .into_iter()
            .map(|r| Face {
                bbox: [r.x, r.y, r.width, r.height],
                name: None,
                rect: (
                    r.x.max(0) as u32,
                    r.y.max(0) as u32,
                    r.width as u32,
                    r.height as u32,
                ),
            })
            .collect();
        Ok(faces)
    }

    /// Associate detected faces with a `MediaItem` in the cache.
    #[cfg(feature = "cache")]
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, cache, item, faces)))]
    pub fn assign_to_cache(
        &self,
        cache: &CacheManager,
        item: &MediaItem,
        faces: &[Face],
    ) -> Result<(), FaceRecognitionError> {
        let json = serde_json::to_string(faces)
            .map_err(|e| FaceRecognitionError::Other(e.to_string()))?;
        cache
            .insert_faces(&item.id, &json)
            .map_err(|e| FaceRecognitionError::CacheError(e.to_string()))
    }

    /// Prepare face data for display in the UI.
    #[cfg(feature = "ui")]
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, faces)))]
    pub fn prepare_ui(&self, faces: &[Face]) -> Vec<UiFace> {
        faces
            .iter()
            .map(|f| UiFace {
                bbox: f.bbox,
                name: f.name.clone(),
            })
            .collect()
    }
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct UiFace {
    pub bbox: [i32; 4],
    pub name: Option<String>,
}
