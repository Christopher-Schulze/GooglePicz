//! User Interface module for GooglePicz.

mod image_loader;
mod video_downloader;
#[path = "../app/src/config.rs"]
mod app_config;
mod style;
mod icon;
mod search;
mod album_dialogs;
mod settings;
mod face_recognizer;

pub use icon::{Icon, MaterialSymbol};
pub use search::SearchMode;
pub use album_dialogs::AlbumOption;
pub use face_recognizer::FaceRecognizer;

pub use image_loader::{ImageLoader, ImageLoaderError};
pub use video_downloader::{VideoDownloader, VideoDownloadError};

use api_client::{Album, ApiClient, MediaItem};
use app_config::AppConfig;
use auth;
use cache::CacheManager;
use google_material_symbols;
use crate::style::{self, Palette};
use face_recognition;
use chrono::{DateTime, Utc};
use iced::subscription;
use iced::widget::container::Appearance;
use iced::widget::image::Handle;
use iced::widget::{
    button, checkbox, column, container, image, pick_list, progress_bar, row,
    scrollable, slider, text, text_input, Column,
};
use iced::Border;
use iced::Color;
use iced::{event, keyboard, executor, Application, Command, Element, Length, Settings, Subscription, Theme};
use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use sync::{SyncProgress, SyncTaskError};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use rfd::AsyncFileDialog;
use sysinfo::{SystemExt, System};
#[cfg(feature = "gstreamer")]
use gstreamer_iced::{GstreamerIcedBase, GStreamerMessage, PlayStatus};
#[cfg(feature = "gstreamer")]
use gstreamer_iced::reexport::url;
#[cfg(feature = "gstreamer")]
use gstreamer as gst;
#[cfg(feature = "gstreamer")]
use tempfile::TempPath;

const ERROR_DISPLAY_DURATION: Duration = Duration::from_secs(5);
const PAGE_SIZE: usize = 40;

fn error_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(|_theme: &Theme| Appearance {
        text_color: Some(Palette::ERROR),
        background: Some(Color { a: 1.0, ..Palette::ERROR }.into()),
        border: Border {
            color: Palette::ERROR,
            width: 1.0,
            radius: 2.0.into(),
        },
        shadow: Default::default(),
    }))
}

#[cfg_attr(feature = "trace-spans", tracing::instrument(skip(progress, errors)))]
pub fn run(
    progress: Option<mpsc::UnboundedReceiver<SyncProgress>>,
    errors: Option<mpsc::UnboundedReceiver<SyncTaskError>>,
    status: Option<mpsc::UnboundedReceiver<SyncTaskError>>,
    preload: usize,
    preload_threads: usize,
    cache_dir: PathBuf,
) -> iced::Result {
    use std::borrow::Cow;
    #[cfg(feature = "trace-spans")]
    let start = std::time::Instant::now();
    #[cfg(feature = "trace-spans")]
    let mut sys = System::new();
    #[cfg(feature = "trace-spans")]
    sys.refresh_memory();
    #[cfg(feature = "trace-spans")]
    let mem_before = sys.used_memory();
    let mut settings = Settings::with_flags((progress, errors, status, preload, preload_threads, cache_dir));
    settings.fonts.push(Cow::Borrowed(google_material_symbols::FONT_BYTES));
    let res = GooglePiczUI::run(settings);
    #[cfg(feature = "trace-spans")]
    {
        sys.refresh_memory();
        let mem_after = sys.used_memory();
        tracing::info!(target = "ui", "startup_time_ms" = start.elapsed().as_millis(), "mem_before_kb" = mem_before, "mem_after_kb" = mem_after);
    }
    res
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadPhotos,
    PhotosLoaded(Result<Vec<MediaItem>, String>),
    LoadAlbums,
    AlbumsLoaded(Result<Vec<Album>, String>),
    RefreshPhotos,
    ThumbnailLoaded(String, Result<Handle, String>),
    LoadThumbnail(String, String), // media_id, base_url
    LoadFullImage(String, String),
    FullImageLoaded(String, Result<Handle, String>),
    LoadFaces(String),
    FacesLoaded(String, Result<Vec<face_recognition::Face>, String>),
    StartRenameFace(usize),
    FaceNameChanged(String),
    SaveFaceName,
    CancelFaceName,
    SelectPhoto(MediaItem),
    SelectAlbum(Option<String>),
    ClosePhoto,
    SyncProgress(SyncProgress),
    SyncError(SyncTaskError),
    SyncStatusUpdated(DateTime<Utc>, String),
    DismissError(usize),
    ShowCreateAlbumDialog,
    AlbumTitleChanged(String),
    CreateAlbum,
    AlbumCreated(Result<Album, String>),
    CancelCreateAlbum,
    AlbumPicked(AlbumOption),
    AlbumAssigned(Result<(), String>),
    RenameAlbum(String, String),
    DeleteAlbum(String),
    ShowRenameAlbumDialog(String, String),
    RenameAlbumTitleChanged(String),
    ConfirmRenameAlbum,
    CancelRenameAlbum,
    ShowDeleteAlbumDialog(String),
    ConfirmDeleteAlbum,
    CancelDeleteAlbum,
    SearchInputChanged(String),
    SearchModeChanged(SearchMode),
    SearchCameraChanged(String),
    SearchCameraMakeChanged(Option<String>),
    SearchMimeChanged(Option<String>),
    SearchStartChanged(String),
    SearchEndChanged(String),
    SearchFavoriteToggled(bool),
    PerformSearch,
    #[cfg(feature = "gstreamer")]
    PlayVideo(MediaItem),
    #[cfg(feature = "gstreamer")]
    VideoEvent(GStreamerMessage),
    #[cfg(feature = "gstreamer")]
    CloseVideo,
    #[cfg(feature = "gstreamer")]
    ToggleVideoPlay,
    #[cfg(feature = "gstreamer")]
    SeekVideo(f64),
    #[cfg(feature = "gstreamer")]
    VideoDownloaded(tempfile::TempPath),
    #[cfg(feature = "gstreamer")]
    VideoDownloadFailed(String),
    ClearErrors,
    ShowSettings,
    CloseSettings,
    SettingsLogLevelChanged(String),
    SettingsCachePathChanged(String),
    SettingsOauthPortChanged(String),
    SettingsThumbsPreloadChanged(String),
    SettingsPreloadThreadsChanged(String),
    SettingsSyncIntervalChanged(String),
    SettingsDebugConsoleToggled(bool),
    SettingsTraceSpansToggled(bool),
    SaveSettings,
    ChooseCachePath,
    CachePathChosen(Option<String>),
    LoadMorePhotos,
    EscapePressed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumOption {
    id: String,
    title: String,
}

impl std::fmt::Display for AlbumOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Filename,
    Description,
    Text,
    Favoriten,
    DateRange,
    MimeType,
    CameraModel,
    CameraMake,
}

impl SearchMode {
    const ALL: [SearchMode; 8] = [
        SearchMode::Filename,
        SearchMode::Description,
        SearchMode::Text,
        SearchMode::Favoriten,
        SearchMode::DateRange,
        SearchMode::MimeType,
        SearchMode::CameraModel,
        SearchMode::CameraMake,
    ];
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SearchMode::Filename => "Filename",
            SearchMode::Description => "Beschreibung",
            SearchMode::Text => "Dateiname/Beschr.",
            SearchMode::Favoriten => "Favoriten",
            SearchMode::DateRange => "Datum von/bis",
            SearchMode::MimeType => "Dateityp",
            SearchMode::CameraModel => "Kamera-Modell",
            SearchMode::CameraMake => "Kamera-Hersteller",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
enum ViewState {
    Grid,
    SelectedPhoto {
        photo: MediaItem,
        faces: Vec<face_recognition::Face>,
    },
    #[cfg(feature = "gstreamer")]
    PlayingVideo {
        player: GstreamerIcedBase,
        file: TempPath,
    },
}

pub struct GooglePiczUI {
    photos: Vec<MediaItem>,
    albums: Vec<Album>,
    loading: bool,
    cache_manager: Option<Arc<Mutex<CacheManager>>>,
    image_loader: Arc<Mutex<ImageLoader>>,
    thumbnails: std::collections::HashMap<String, Handle>,
    full_images: std::collections::HashMap<String, Handle>,
    progress_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<SyncProgress>>>>,
    error_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<SyncTaskError>>>>,
    status_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<SyncTaskError>>>>,
    synced: u64,
    syncing: bool,
    last_synced: Option<DateTime<Utc>>,
    sync_status: String,
    state: ViewState,
    selected_album: Option<String>,
    errors: Vec<String>,
    preload_count: usize,
    display_limit: usize,
    creating_album: bool,
    new_album_title: String,
    assign_selection: Option<AlbumOption>,
    renaming_album: Option<String>,
    rename_album_title: String,
    deleting_album: Option<String>,
    search_mode: SearchMode,
    search_query: String,
    search_camera: String,
    search_camera_make: Option<String>,
    search_mime: Option<String>,
    camera_make_options: Vec<String>,
    mime_options: Vec<String>,
    search_start: String,
    search_end: String,
    search_favorite: bool,
    error_log_path: PathBuf,
    settings_open: bool,
    config_path: PathBuf,
    settings_log_level: String,
    settings_cache_path: String,
    settings_oauth_port: String,
    settings_thumbnails_preload: String,
    settings_preload_threads: String,
    settings_sync_interval: String,
    settings_debug_console: bool,
    settings_trace_spans: bool,
    editing_face: Option<usize>,
    face_name_input: String,
}

impl GooglePiczUI {
    /// Expose current state for testing purposes
    pub fn state_debug(&self) -> String {
        format!("{:?}", self.state)
    }

    /// Return number of stored errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn photo_count(&self) -> usize {
        self.photos.len()
    }

    pub fn album_count(&self) -> usize {
        self.albums.len()
    }

    pub fn renaming_album(&self) -> Option<String> {
        self.renaming_album.clone()
    }

    pub fn deleting_album(&self) -> Option<String> {
        self.deleting_album.clone()
    }

    pub fn search_query(&self) -> String {
        self.search_query.clone()
    }

    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    pub fn rename_album_title(&self) -> String {
        self.rename_album_title.clone()
    }

    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    pub fn settings_log_level(&self) -> String {
        self.settings_log_level.clone()
    }

    pub fn settings_cache_path(&self) -> String {
        self.settings_cache_path.clone()
    }

    pub fn settings_oauth_port(&self) -> String {
        self.settings_oauth_port.clone()
    }

    pub fn settings_thumbnails_preload(&self) -> String {
        self.settings_thumbnails_preload.clone()
    }

    pub fn settings_preload_threads(&self) -> String {
        self.settings_preload_threads.clone()
    }

    pub fn settings_sync_interval(&self) -> String {
        self.settings_sync_interval.clone()
    }

    pub fn settings_debug_console(&self) -> bool {
        self.settings_debug_console
    }

    pub fn settings_trace_spans(&self) -> bool {
        self.settings_trace_spans
    }

    pub fn sync_status(&self) -> String {
        self.sync_status.clone()
    }

    pub fn syncing(&self) -> bool {
        self.syncing
    }

    pub fn face_count(&self) -> usize {
        match &self.state {
            ViewState::SelectedPhoto { faces, .. } => faces.len(),
            _ => 0,
        }
    }

    pub fn face_name(&self, idx: usize) -> Option<String> {
        match &self.state {
            ViewState::SelectedPhoto { faces, .. } => faces.get(idx).and_then(|f| f.name.clone()),
            _ => None,
        }
    }

    pub fn editing_face(&self) -> Option<usize> {
        self.editing_face
    }
    fn log_error(&self, msg: &str) {
        tracing::error!("{}", msg);
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.error_log_path)
        {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", msg) {
                    tracing::error!(error = ?e, "Failed to write to error log");
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to open error log file");
            }
        }
    }
    fn error_timeout() -> Command<Message> {
        Command::perform(
            async {
                sleep(ERROR_DISPLAY_DURATION).await;
            },
            |_| Message::ClearErrors,
        )
    }
}

impl Application for GooglePiczUI {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = (
        Option<mpsc::UnboundedReceiver<SyncProgress>>,
        Option<mpsc::UnboundedReceiver<SyncTaskError>>,
        Option<mpsc::UnboundedReceiver<SyncTaskError>>,
        usize,
        usize,
        PathBuf,
    );

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(flags)))]
    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (progress_flag, error_flag, status_flag, preload_count, preload_threads, cache_dir) = flags;
        #[cfg(feature = "trace-spans")]
        let start = std::time::Instant::now();
        #[cfg(feature = "trace-spans")]
        let mut sys = System::new();
        #[cfg(feature = "trace-spans")]
        sys.refresh_memory();
        #[cfg(feature = "trace-spans")]
        let mem_before = sys.used_memory();
        let mut init_errors = Vec::new();
        let error_log_path = cache_dir.join("ui_errors.log");
        let cache_path = cache_dir.join("cache.sqlite");
        let config_path = cache_dir.join("config");

        #[cfg(feature = "gstreamer")]
        if let Err(e) = gst::init() {
            init_errors.push(format!("GStreamer initialization failed: {}", e));
        }

        let cache_manager = match CacheManager::new(&cache_path) {
            Ok(cm) => Some(Arc::new(Mutex::new(cm))),
            Err(e) => {
                let msg = format!("Failed to initialize cache: {}", e);
                init_errors.push(msg.clone());
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&error_log_path)
                {
                    let _ = writeln!(f, "{}", msg);
                }
                None
            }
        };

        let last_synced = if let Some(cm) = &cache_manager {
            let cache = cm.blocking_lock();
            match cache.get_last_sync() {
                Ok(ts) => Some(ts),
                Err(e) => {
                    let msg = format!("Failed to read last sync: {}", e);
                    init_errors.push(msg.clone());
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&error_log_path)
                    {
                        if let Err(e) = writeln!(f, "{}", msg) {
                            tracing::error!(error = ?e, "Failed to write to error log");
                        }
                    } else {
                        tracing::error!("Failed to open error log file");
                    }
                    None
                }
            }
        } else {
            None
        };

        let image_loader = Arc::new(Mutex::new(ImageLoader::new(cache_dir.clone(), preload_threads)));

        let progress_receiver = progress_flag.map(|rx| Arc::new(Mutex::new(rx)));
        let error_receiver = error_flag.map(|rx| Arc::new(Mutex::new(rx)));
        let status_receiver = status_flag.map(|rx| Arc::new(Mutex::new(rx)));

        let status = match last_synced {
            Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
            None => "Never synced".to_string(),
        };

        let cfg = AppConfig::load_from(Some(config_path.clone()));
        let open_settings = std::env::var("OPEN_SETTINGS").unwrap_or_default() == "1";

        let app = Self {
            photos: Vec::new(),
            albums: Vec::new(),
            loading: false,
            cache_manager,
            image_loader,
            thumbnails: std::collections::HashMap::new(),
            full_images: std::collections::HashMap::new(),
            progress_receiver,
            error_receiver,
            status_receiver,
            synced: 0,
            syncing: false,
            last_synced,
            sync_status: status,
            state: ViewState::Grid,
            selected_album: None,
            errors: init_errors,
            preload_count,
            display_limit: 0,
            creating_album: false,
            new_album_title: String::new(),
            assign_selection: None,
            renaming_album: None,
            rename_album_title: String::new(),
            deleting_album: None,
            search_mode: SearchMode::Filename,
            search_query: String::new(),
            search_camera: String::new(),
            search_camera_make: None,
            search_mime: None,
            camera_make_options: Vec::new(),
            mime_options: Vec::new(),
            search_start: String::new(),
            search_end: String::new(),
            search_favorite: false,
            error_log_path,
            settings_open: open_settings,
            config_path,
            settings_log_level: cfg.log_level.clone(),
            settings_cache_path: cfg.cache_path.to_string_lossy().to_string(),
            settings_oauth_port: cfg.oauth_redirect_port.to_string(),
            settings_thumbnails_preload: cfg.thumbnails_preload.to_string(),
            settings_preload_threads: cfg.preload_threads.to_string(),
            settings_sync_interval: cfg.sync_interval_minutes.to_string(),
            settings_debug_console: cfg.debug_console,
            settings_trace_spans: cfg.trace_spans,
            editing_face: None,
            face_name_input: String::new(),
        };
        #[cfg(feature = "trace-spans")]
        {
            sys.refresh_memory();
            tracing::info!(target = "ui", "init_time_ms" = start.elapsed().as_millis(), "mem_before_kb" = mem_before, "mem_after_kb" = sys.used_memory());
        }

        (
            app,
            Command::batch(vec![
                Command::perform(async {}, |_| Message::LoadPhotos),
                Command::perform(async {}, |_| Message::LoadAlbums),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("GooglePicz - Google Photos Manager")
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LoadPhotos => {
                self.loading = true;
                if let Some(album_id) = &self.selected_album {
                    let album_id = album_id.clone();
                    return Command::perform(
                        async move {
                            let token = auth::ensure_access_token_valid()
                                .await
                                .map_err(|e| e.to_string())?;
                            let client = ApiClient::new(token);
                            client
                                .get_album_media_items(&album_id, 100, None)
                                .await
                                .map(|r| r.0)
                                .map_err(|e| e.to_string())
                        },
                        Message::PhotosLoaded,
                    );
                } else if let Some(cache_manager) = &self.cache_manager {
                    let cache_manager = cache_manager.clone();
                    return Command::perform(
                        async move {
                            let cache = {
                                let guard = cache_manager.lock().await;
                                guard.clone()
                            };
                            cache
                                .get_all_media_items_async()
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::PhotosLoaded,
                    );
                }
            }
            Message::PhotosLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(photos) => {
                        self.photos = photos;
                        use std::collections::HashSet;
                        let mut mimes: HashSet<String> = HashSet::new();
                        let mut makes: HashSet<String> = HashSet::new();
                        for photo in &self.photos {
                            mimes.insert(photo.mime_type.clone());
                            if let Some(make) = photo
                                .media_metadata
                                .video
                                .as_ref()
                                .and_then(|v| v.camera_make.clone())
                            {
                                makes.insert(make);
                            }
                        }
                        self.mime_options = mimes.into_iter().collect();
                        self.mime_options.sort();
                        self.camera_make_options = makes.into_iter().collect();
                        self.camera_make_options.sort();
                        self.display_limit = PAGE_SIZE.min(self.photos.len());
                        // Start loading thumbnails for configured number of photos
                        let mut commands = Vec::new();
                        for photo in self.photos.iter().take(self.preload_count) {
                            let media_id = photo.id.clone();
                            let base_url = photo.base_url.clone();
                            commands.push(Command::perform(async {}, move |_| {
                                Message::LoadThumbnail(media_id.clone(), base_url.clone())
                            }));
                        }
                        return Command::batch(commands);
                    }
                    Err(error) => {
                        self.errors
                            .push(format!("Failed to load photos: {}", error));
                        return GooglePiczUI::error_timeout();
                    }
                }
            }
            Message::LoadAlbums => {
                return Command::perform(
                    async {
                        let token = auth::ensure_access_token_valid()
                            .await
                            .map_err(|e| e.to_string())?;
                        let client = ApiClient::new(token);
                        client
                            .list_albums(50, None)
                            .await
                            .map(|r| r.0)
                            .map_err(|e| e.to_string())
                    },
                    Message::AlbumsLoaded,
                );
            }
            Message::AlbumsLoaded(result) => match result {
                Ok(albums) => {
                    self.albums = albums;
                }
                Err(err) => {
                    let msg = format!("Failed to load albums: {}", err);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
            },
            Message::RefreshPhotos => {
                return Command::batch(vec![
                    Command::perform(async {}, |_| Message::LoadPhotos),
                    Command::perform(async {}, |_| Message::LoadAlbums),
                ]);
            }
            Message::LoadMorePhotos => {
                self.display_limit = (self.display_limit + PAGE_SIZE).min(self.photos.len());
            }
            Message::LoadThumbnail(media_id, base_url) => {
                let image_loader = self.image_loader.clone();
                let id_clone = media_id.clone();
                let base_clone = base_url.clone();
                return Command::perform(
                    async move {
                        let loader = image_loader.lock().await;
                        loader.load_thumbnail(&id_clone, &base_clone).await
                    },
                    move |result| {
                        Message::ThumbnailLoaded(media_id, result.map_err(|e| e.to_string()))
                    },
                );
            }
            Message::ThumbnailLoaded(media_id, result) => match result {
                Ok(handle) => {
                    self.thumbnails.insert(media_id, handle);
                }
                Err(error) => {
                    let msg = format!("Failed to load thumbnail for {}: {}", media_id, error);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
            },
            Message::SelectPhoto(photo) => {
                let id = photo.id.clone();
                let url = photo.base_url.clone();
                self.state = ViewState::SelectedPhoto { photo, faces: Vec::new() };
                return Command::batch(vec![
                    Command::perform(async {}, move |_| {
                        Message::LoadFullImage(id.clone(), url.clone())
                    }),
                    Command::perform(async {}, move |_| Message::LoadFaces(id.clone())),
                ]);
            }
            Message::SelectAlbum(album_id) => {
                self.selected_album = album_id;
                return Command::perform(async {}, |_| Message::LoadPhotos);
            }
            Message::LoadFullImage(media_id, base_url) => {
                let loader = self.image_loader.clone();
                let id_clone = media_id.clone();
                let base_clone = base_url.clone();
                return Command::perform(
                    async move {
                        let loader = loader.lock().await;
                        loader.load_full_image(&id_clone, &base_clone).await
                    },
                    move |res| Message::FullImageLoaded(media_id, res.map_err(|e| e.to_string())),
                );
            }
            Message::LoadFaces(media_id) => {
                if let Some(cm) = &self.cache_manager {
                    let cm = cm.clone();
                    let id_clone = media_id.clone();
                    return Command::perform(
                        async move {
                            let cache = { let guard = cm.lock().await; guard.clone() };
                            cache.get_faces_for_media_item(&id_clone).await.map_err(|e| e.to_string())
                        },
                        move |res| Message::FacesLoaded(media_id, res),
                    );
                }
            }
            Message::FullImageLoaded(media_id, result) => match result {
                Ok(handle) => {
                    self.full_images.insert(media_id, handle);
                }
                Err(error) => {
                    let msg = format!("Failed to load image: {}", error);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
            },
            Message::FacesLoaded(media_id, result) => {
                if let ViewState::SelectedPhoto { photo, faces } = &mut self.state {
                    if photo.id == media_id {
                        match result {
                            Ok(v) => *faces = v,
                            Err(e) => {
                                #[cfg(feature = "face_recognition")]
                                {
                                    self.errors.push(e.clone());
                                    self.log_error(&e);
                                }
                                #[cfg(not(feature = "face_recognition"))]
                                {
                                    let msg = format!("Failed to load faces: {}", e);
                                    self.errors.push(msg.clone());
                                    self.log_error(&msg);
                                }
                                return GooglePiczUI::error_timeout();
                            }
                        }
                    }
                }
            }
            Message::StartRenameFace(idx) => {
                self.editing_face = Some(idx);
                if let ViewState::SelectedPhoto { faces, .. } = &self.state {
                    if let Some(f) = faces.get(idx) {
                        self.face_name_input = f.name.clone().unwrap_or_default();
                    }
                }
            }
            Message::FaceNameChanged(name) => {
                self.face_name_input = name;
            }
            Message::SaveFaceName => {
                if let Some(idx) = self.editing_face.take() {
                    if let ViewState::SelectedPhoto { faces, photo } = &mut self.state {
                        if let Some(face) = faces.get_mut(idx) {
                            face.name = Some(self.face_name_input.clone());
                        }
                        if let Some(cm) = &self.cache_manager {
                            let cm = cm.clone();
                            let media_id = photo.id.clone();
                            let name = self.face_name_input.clone();
                            return Command::perform(
                                async move {
                                    let cache = { let guard = cm.lock().await; guard.clone() };
                                    cache.update_face_name(&media_id, idx, &name).await.map_err(|e| e.to_string())
                                },
                                |_| Message::CancelFaceName,
                            );
                        }
                    }
                }
                self.face_name_input.clear();
            }
            Message::CancelFaceName => {
                self.editing_face = None;
                self.face_name_input.clear();
            }
            Message::ClosePhoto => {
                self.state = ViewState::Grid;
            }
            #[cfg(feature = "gstreamer")]
            Message::PlayVideo(item) => {
                let url = format!("{}=dv", item.base_url);
                return Command::perform(
                    async move {
                        VideoDownloader::new()
                            .download_to_tempfile(&url, ".mp4")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    |res| match res {
                        Ok(p) => Message::VideoDownloaded(p),
                        Err(e) => Message::VideoDownloadFailed(e),
                    },
                );
            }
            #[cfg(feature = "gstreamer")]
            Message::VideoDownloaded(temp) => {
                match url::Url::from_file_path(&temp) {
                    Ok(u) => match GstreamerIcedBase::new_url(&u, false) {
                        Ok(mut player) => {
                            let _ = player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Playing));
                            self.state = ViewState::PlayingVideo { player, file: temp };
                        }
                        Err(e) => {
                            let detail = e.to_string();
                            let msg = if detail.to_lowercase().contains("initialize") {
                                "GStreamer not available".to_string()
                            } else {
                                format!("Failed to start video: {detail}. Missing codecs?")
                            };
                            self.errors.push(msg.clone());
                            self.log_error(&msg);
                            drop(temp); // ensure temp file cleanup
                            return GooglePiczUI::error_timeout();
                        }
                    },
                    Err(_) => {
                        let msg = "Invalid video file path".to_string();
                        self.errors.push(msg.clone());
                        self.log_error(&msg);
                        drop(temp);
                        return GooglePiczUI::error_timeout();
                    }
                }
            }
            #[cfg(feature = "gstreamer")]
            Message::VideoDownloadFailed(err) => {
                self.errors.push(err.clone());
                self.log_error(&err);
                return GooglePiczUI::error_timeout();
            }
            #[cfg(feature = "gstreamer")]
            Message::VideoEvent(msg) => {
                if let ViewState::PlayingVideo { player, .. } = &mut self.state {
                    if let GStreamerMessage::BusGoToEnd = msg {
                        self.state = ViewState::Grid;
                        return Command::none();
                    }
                    return player.update(msg).map(Message::VideoEvent);
                }
            }
            #[cfg(feature = "gstreamer")]
            Message::ToggleVideoPlay => {
                if let ViewState::PlayingVideo { player, .. } = &mut self.state {
                    let new_status = if matches!(player.play_status(), PlayStatus::Playing) {
                        PlayStatus::Stop
                    } else {
                        PlayStatus::Playing
                    };
                    return player
                        .update(GStreamerMessage::PlayStatusChanged(new_status))
                        .map(Message::VideoEvent);
                }
            }
            #[cfg(feature = "gstreamer")]
            Message::SeekVideo(pos) => {
                if let ViewState::PlayingVideo { player, .. } = &mut self.state {
                    let _ = player.seek(std::time::Duration::from_secs_f64(pos));
                }
            }
            #[cfg(feature = "gstreamer")]
            Message::CloseVideo => {
                self.state = ViewState::Grid;
            }
            Message::SyncProgress(progress) => match progress {
                SyncProgress::Started => {
                    self.synced = 0;
                    self.syncing = true;
                    self.sync_status = "Sync started".into();
                }
                SyncProgress::Retrying(wait) => {
                    self.syncing = false;
                    self.sync_status = format!("Retrying in {}s", wait);
                }
                SyncProgress::ItemSynced(count) => {
                    self.synced = count;
                    self.syncing = true;
                    self.sync_status = format!("Syncing {} items", count);
                }
                SyncProgress::Finished(total) => {
                    self.synced = total;
                    self.syncing = false;
                    self.last_synced = Some(Utc::now());
                    self.sync_status = format!("Sync completed: {} items", total);
                }
            },
            Message::SyncStatusUpdated(ts, message) => {
                self.last_synced = Some(ts);
                self.sync_status = message.clone();
                if message.contains("Sync started") || message.contains("Syncing") {
                    self.syncing = true;
                } else if message.contains("Sync completed") {
                    self.syncing = false;
                }
            },
            Message::SyncError(err_msg) => {
                match err_msg {
                    other => {
                        tracing::error!("Sync error: {}", other);
                        let detail = match &other {
                            SyncTaskError::TokenRefreshFailed { message, .. }
                            | SyncTaskError::PeriodicSyncFailed { message, .. }
                            | SyncTaskError::Other { message, .. }
                            | SyncTaskError::Aborted(message) => message.clone(),
                            SyncTaskError::RestartAttempt(attempt) => format!("Restart attempt {attempt}"),
                        };
                        if let Some(idx) = detail.find("last_success:") {
                            let ts_str = detail[idx + "last_success:".len()..].trim();
                            if let Some(end) = ts_str.split_whitespace().next() {
                                if let Ok(dt) = DateTime::parse_from_rfc3339(end) {
                                    self.last_synced = Some(dt.with_timezone(&Utc));
                                }
                            }
                        }
                        self.errors.push(other.to_string());
                        self.log_error(&other.to_string());
                        self.sync_status = "Sync error".into();
                        self.syncing = false;
                        return GooglePiczUI::error_timeout();
                    }
                }
            }
            Message::DismissError(index) => {
                if index < self.errors.len() {
                    self.errors.remove(index);
                }
            }
            Message::ClearErrors => {
                self.errors.clear();
            }
            Message::ShowSettings => {
                self.settings_open = true;
                let cfg = AppConfig::load_from(Some(self.config_path.clone()));
                self.settings_log_level = cfg.log_level;
                self.settings_cache_path = cfg.cache_path.to_string_lossy().to_string();
                self.settings_oauth_port = cfg.oauth_redirect_port.to_string();
                self.settings_thumbnails_preload = cfg.thumbnails_preload.to_string();
                self.settings_preload_threads = cfg.preload_threads.to_string();
                self.settings_sync_interval = cfg.sync_interval_minutes.to_string();
                self.settings_debug_console = cfg.debug_console;
                self.settings_trace_spans = cfg.trace_spans;
            }
            Message::CloseSettings => {
                self.settings_open = false;
            }
            Message::SettingsLogLevelChanged(val) => {
                self.settings_log_level = val;
            }
            Message::SettingsCachePathChanged(val) => {
                self.settings_cache_path = val;
            }
            Message::SettingsOauthPortChanged(val) => {
                self.settings_oauth_port = val;
            }
            Message::SettingsThumbsPreloadChanged(val) => {
                self.settings_thumbnails_preload = val;
            }
            Message::SettingsPreloadThreadsChanged(val) => {
                self.settings_preload_threads = val;
            }
            Message::SettingsSyncIntervalChanged(val) => {
                self.settings_sync_interval = val;
            }
            Message::SettingsDebugConsoleToggled(val) => {
                self.settings_debug_console = val;
            }
            Message::SettingsTraceSpansToggled(val) => {
                self.settings_trace_spans = val;
            }
            Message::ChooseCachePath => {
                return Command::perform(async {
                    AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|f| f.path().to_path_buf())
                }, Message::CachePathChosen);
            }
            Message::CachePathChosen(opt) => {
                if let Some(p) = opt {
                    self.settings_cache_path = p.to_string_lossy().to_string();
                }
            }
            Message::SaveSettings => {
                let mut cfg = AppConfig::load_from(Some(self.config_path.clone()));
                cfg.log_level = self.settings_log_level.clone();
                cfg.cache_path = PathBuf::from(self.settings_cache_path.clone());
                if let Ok(p) = self.settings_oauth_port.parse() {
                    cfg.oauth_redirect_port = p;
                }
                if let Ok(t) = self.settings_thumbnails_preload.parse() {
                    cfg.thumbnails_preload = t;
                }
                if let Ok(t) = self.settings_preload_threads.parse() {
                    cfg.preload_threads = t;
                }
                if let Ok(s) = self.settings_sync_interval.parse() {
                    cfg.sync_interval_minutes = s;
                }
                cfg.debug_console = self.settings_debug_console;
                cfg.trace_spans = self.settings_trace_spans;
                if let Err(e) = cfg.save_to(Some(self.config_path.clone())) {
                    let msg = format!("Failed to save settings: {}", e);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
                self.settings_open = false;
            }
            Message::ShowCreateAlbumDialog => {
                self.creating_album = true;
            }
            Message::AlbumTitleChanged(title) => {
                self.new_album_title = title;
            }
            Message::CreateAlbum => {
                let title = self.new_album_title.clone();
                self.new_album_title.clear();
                self.creating_album = false;
                let cache_manager = self.cache_manager.clone();
                return Command::perform(
                    async move {
                        let token = auth::ensure_access_token_valid()
                            .await
                            .map_err(|e| e.to_string())?;
                        let client = ApiClient::new(token);
                        let album = client
                            .create_album(&title)
                            .await
                            .map_err(|e| e.to_string())?;
                        if let Some(cm) = cache_manager {
                            let cache = {
                                let guard = cm.lock().await;
                                guard.clone()
                            };
                            if let Err(e) = cache.insert_album_async(album.clone()).await {
                                return Err(e.to_string());
                            }
                        }
                        Ok(album)
                    },
                    Message::AlbumCreated,
                );
            }
            Message::AlbumCreated(result) => match result {
                Ok(album) => {
                    self.albums.push(album);
                }
                Err(err) => {
                    let msg = format!("Failed to create album: {}", err);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
            },
            Message::CancelCreateAlbum => {
                self.creating_album = false;
                self.new_album_title.clear();
            }
            Message::AlbumPicked(album) => {
                self.assign_selection = Some(album.clone());
                if let ViewState::SelectedPhoto { photo, .. } = &self.state {
                    if let Some(cm) = &self.cache_manager {
                        let cm = cm.clone();
                        let media_id = photo.id.clone();
                        let album_id = album.id.clone();
                        return Command::perform(
                            async move {
                                let cache = {
                                    let guard = cm.lock().await;
                                    guard.clone()
                                };
                                cache
                                    .associate_media_item_with_album_async(media_id.clone(), album_id.clone())
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                            Message::AlbumAssigned,
                        );
                    }
                }
            }
            Message::AlbumAssigned(res) => {
                self.assign_selection = None;
                if let Err(e) = res {
                    let msg = format!("Failed to assign photo: {}", e);
                    self.errors.push(msg.clone());
                    self.log_error(&msg);
                    return GooglePiczUI::error_timeout();
                }
            }
            Message::ShowRenameAlbumDialog(id, title) => {
                self.renaming_album = Some(id);
                self.rename_album_title = title;
            }
            Message::RenameAlbumTitleChanged(t) => {
                self.rename_album_title = t;
            }
            Message::ConfirmRenameAlbum => {
                if let Some(id) = self.renaming_album.take() {
                    let title = self.rename_album_title.clone();
                    self.rename_album_title.clear();
                    return self.update(Message::RenameAlbum(id, title));
                }
            }
            Message::CancelRenameAlbum => {
                self.renaming_album = None;
                self.rename_album_title.clear();
            }
            Message::ShowDeleteAlbumDialog(id) => {
                self.deleting_album = Some(id);
            }
            Message::ConfirmDeleteAlbum => {
                if let Some(id) = self.deleting_album.take() {
                    return self.update(Message::DeleteAlbum(id));
                }
            }
            Message::CancelDeleteAlbum => {
                self.deleting_album = None;
            }
            Message::EscapePressed => {
                if self.settings_open {
                    return self.update(Message::CloseSettings);
                }
                if self.renaming_album.is_some() {
                    return self.update(Message::CancelRenameAlbum);
                }
                if self.deleting_album.is_some() {
                    return self.update(Message::CancelDeleteAlbum);
                }
                if self.editing_face.is_some() {
                    return self.update(Message::CancelFaceName);
                }
                if let ViewState::SelectedPhoto { .. } = &self.state {
                    self.state = ViewState::Grid;
                }
            }
            Message::SearchInputChanged(q) => {
                self.search_query = q;
            }
            Message::SearchModeChanged(mode) => {
                self.search_mode = mode;
            }
            Message::SearchCameraChanged(v) => {
                self.search_camera = v;
            }
            Message::SearchCameraMakeChanged(v) => {
                self.search_camera_make = v;
            }
            Message::SearchMimeChanged(v) => {
                self.search_mime = v;
            }
            Message::SearchStartChanged(v) => {
                self.search_start = v;
            }
            Message::SearchEndChanged(v) => {
                self.search_end = v;
            }
            Message::SearchFavoriteToggled(v) => {
                self.search_favorite = v;
            }
            Message::PerformSearch => {
                if let Some(cm) = &self.cache_manager {
                    let cm = cm.clone();
                    let query = self.search_query.clone();
                    let mode = self.search_mode;
                    let camera = self.search_camera.clone();
                    let start = self.search_start.clone();
                    let end = self.search_end.clone();
                    let fav = self.search_favorite;
                    let make_sel = self.search_camera_make.clone();
                    let mime_sel = self.search_mime.clone();
                    return Command::perform(
                        async move {
                            let cache = {
                                let guard = cm.lock().await;
                                guard.clone()
                            };
                            let base = match mode {
                                SearchMode::Filename => Some(
                                    cache
                                        .get_media_items_by_filename(&query)
                                        .map_err(|e| e.to_string())?,
                                ),
                                SearchMode::Description => Some(
                                    cache
                                        .get_media_items_by_description(&query)
                                        .map_err(|e| e.to_string())?,
                                ),
                                SearchMode::Text => Some(
                                    cache
                                        .get_media_items_by_text(&query)
                                        .map_err(|e| e.to_string())?,
                                ),
                                _ => None,
                            };

                            let start_dt = parse_single_date(&start, false);
                            let end_dt = parse_single_date(&end, true);
                            let camera_model_param = if mode == SearchMode::CameraModel {
                                Some(query.clone())
                            } else if camera.is_empty() {
                                None
                            } else {
                                Some(camera)
                            };
                            let camera_make_param = if mode == SearchMode::CameraMake {
                                Some(query.clone())
                            } else {
                                make_sel.clone()
                            };
                            let mime_param = if mode == SearchMode::MimeType {
                                Some(query.clone())
                            } else {
                                mime_sel.clone()
                            };
                            let fav_param = if mode == SearchMode::Favoriten || fav {
                                Some(true)
                            } else {
                                None
                            };
                                SearchMode::DateRange => {
                                    if let Some((s, e)) = search::parse_date_query(&query) {
                                        cache
                                            .get_media_items_by_date_range(s, e)
                                            .map_err(|e| e.to_string())
                                    } else {
                                        Ok(Vec::new())
                                    }
                                }
                                SearchMode::Description => cache
                                    .get_media_items_by_description(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::Favoriten => cache
                                    .get_favorite_media_items()
                                    .map_err(|e| e.to_string()),
                                SearchMode::Filename => cache
                                    .get_media_items_by_filename(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::MimeType => cache
                                    .get_media_items_by_mime_type(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::CameraModel => cache
                                    .get_media_items_by_camera_model(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::CameraMake => cache
                                    .get_media_items_by_camera_make(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::Text => cache
                                    .get_media_items_by_text(&query)
                                    .map_err(|e| e.to_string()),
                            }?;

                            let start_dt = search::parse_single_date(&start, false);
                            let end_dt = search::parse_single_date(&end, true);
                            cache
                                .query_media_items_async(
                                    camera_model_param,
                                    camera_make_param,
                                    start_dt,
                                    end_dt,
                                    fav_param,
                                    mime_param,
                                    if mode == SearchMode::Text { Some(query.clone()) } else { None },
                                )
                                .await
                                .map_err(|e| e.to_string())
                                .map(|mut extra| {
                                    if let Some(base) = &base {
                                        extra.retain(|i| base.iter().any(|b| b.id == i.id));
                                    }
                                    extra
                                })
                        },
                        Message::PhotosLoaded,
                    );
                }
            }
            Message::RenameAlbum(id, title) => {
                let cache_manager = self.cache_manager.clone();
                return Command::perform(
                    async move {
                        let token = auth::ensure_access_token_valid()
                            .await
                            .map_err(|e| e.to_string())?;
                        let client = ApiClient::new(token);
                        client.rename_album(&id, &title).await.map_err(|e| e.to_string())?;
                        if let Some(cm) = cache_manager {
                            let cache = {
                                let guard = cm.lock().await;
                                guard.clone()
                            };
                            cache
                                .rename_album_async(id.clone(), title.clone())
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |_: Result<_, _>| Message::LoadAlbums,
                );
            }
            Message::DeleteAlbum(id) => {
                let cache_manager = self.cache_manager.clone();
                return Command::perform(
                    async move {
                        let token = auth::ensure_access_token_valid()
                            .await
                            .map_err(|e| e.to_string())?;
                        let client = ApiClient::new(token);
                        client.delete_album(&id).await.map_err(|e| e.to_string())?;
                        if let Some(cm) = cache_manager {
                            let cache = {
                                let guard = cm.lock().await;
                                guard.clone()
                            };
                            cache
                                .delete_album_async(id.clone())
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |_: Result<_, _>| Message::LoadAlbums,
                );
            }
        }
        Command::none()
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    fn subscription(&self) -> Subscription<Message> {
        let mut subs: Vec<Subscription<Message>> = Vec::new();

        if let Some(progress_rx) = &self.progress_receiver {
            let progress_rx = progress_rx.clone();
            subs.push(subscription::unfold("progress", progress_rx, |rx| async move {
                let mut lock = rx.lock().await;
                let msg = match lock.recv().await {
                    Some(p) => Message::SyncProgress(p),
                    None => Message::SyncProgress(SyncProgress::Finished(0)),
                };
                drop(lock);
                (msg, rx)
            }));
        }

        if let Some(error_rx) = &self.error_receiver {
            let error_rx = error_rx.clone();
            subs.push(subscription::unfold("errors", error_rx, |rx| async move {
                let mut lock = rx.lock().await;
                let msg = match lock.recv().await {
                    Some(SyncTaskError::Status { last_synced, message }) => {
                        Message::SyncStatusUpdated(last_synced, message)
                    }
                    Some(e) => Message::SyncError(e),
                    None => Message::SyncProgress(SyncProgress::Finished(0)),
                };
                drop(lock);
                (msg, rx)
            }));
        }

        if let Some(status_rx) = &self.status_receiver {
            let status_rx = status_rx.clone();
            subs.push(subscription::unfold("status", status_rx, |rx| async move {
                let mut lock = rx.lock().await;
                let msg = match lock.recv().await {
                    Some(SyncTaskError::Status { last_synced, message }) => {
                        Message::SyncStatusUpdated(last_synced, message)
                    }
                    Some(e) => Message::SyncError(e),
                    None => Message::SyncProgress(SyncProgress::Finished(0)),
                };
                drop(lock);
                (msg, rx)
            }));
        }

        subs.push(iced::subscription::events().filter_map(|event| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key_code, .. }) => {
                if key_code == iced::keyboard::KeyCode::Escape {
                    Some(Message::EscapePressed)
                } else {
                    None
                }
            }
            _ => None
        }));

        #[cfg(feature = "gstreamer")]
        if let ViewState::PlayingVideo { player, .. } = &self.state {
            subs.push(player.subscription().map(Message::VideoEvent));
        }

        Subscription::batch(subs)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    fn view(&self) -> Element<Message> {
        let placeholder = match self.search_mode {
            SearchMode::Filename => "Filename",
            SearchMode::Description => "Description",
            SearchMode::Text => "Filename or description",
            SearchMode::Favoriten => "Favorites", 
            SearchMode::MimeType => "Mime type",
            SearchMode::CameraModel => "Camera model",
            SearchMode::CameraMake => "Camera make",
            SearchMode::DateRange => "YYYY-MM-DD..YYYY-MM-DD",
        };

        let mut header = row![
            text("GooglePicz").size(24),
            button(Icon::new(MaterialSymbol::Refresh).color(Palette::ON_PRIMARY)).style(style::button_primary()).on_press(Message::RefreshPhotos),
            button(Icon::new(MaterialSymbol::Add).color(Palette::ON_PRIMARY)).style(style::button_primary()).on_press(Message::ShowCreateAlbumDialog),
            button(Icon::new(MaterialSymbol::Settings).color(Palette::ON_PRIMARY)).style(style::button_primary()).on_press(Message::ShowSettings),
            text_input(placeholder, &self.search_query)
                .style(style::text_input())
                .on_input(Message::SearchInputChanged),
            text_input("Camera", &self.search_camera)
                .style(style::text_input())
                .on_input(Message::SearchCameraChanged),
            pick_list(
                &self.camera_make_options,
                self.search_camera_make.clone(),
                Message::SearchCameraMakeChanged,
            ),
            pick_list(
                &self.mime_options,
                self.search_mime.clone(),
                Message::SearchMimeChanged,
            ),
            text_input("From", &self.search_start)
                .style(style::text_input())
                .on_input(Message::SearchStartChanged),
            text_input("To", &self.search_end)
                .style(style::text_input())
                .on_input(Message::SearchEndChanged),
            checkbox("Fav", self.search_favorite, Message::SearchFavoriteToggled)
                .style(style::checkbox_primary()),
            pick_list(
                &SearchMode::ALL[..],
                Some(self.search_mode),
                Message::SearchModeChanged,
            ),
            button(Icon::new(MaterialSymbol::Search).color(Palette::ON_PRIMARY))
                .style(style::button_primary())
                .on_press(Message::PerformSearch)
        ];
        header = header
            .push(search::view(self));

        if let Some(album_id) = &self.selected_album {
            header = header
                .push(
                    button(Icon::new(MaterialSymbol::Edit).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::ShowRenameAlbumDialog(
                        album_id.clone(),
                        self.albums
                            .iter()
                            .find(|a| a.id == *album_id)
                            .and_then(|a| a.title.clone())
                            .unwrap_or_default(),
                    )),
                )
                .push(
                    button(Icon::new(MaterialSymbol::Delete).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::ShowDeleteAlbumDialog(album_id.clone()))
                );
        }

        header = header
            .push(text(self.sync_status.clone()))
            .push(if self.syncing {
                progress_bar(0.0..=1.0, ((self.synced % PAGE_SIZE as u64) as f32) / PAGE_SIZE as f32)
                    .width(Length::Fixed(120.0))
            } else {
                progress_bar(0.0..=1.0, 0.0).width(Length::Fixed(0.0))
            })
            .push(text(match self.last_synced {
                Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
                None => "Never synced".to_string(),
            }))
            .push(text(format!("Errors: {}", self.errors.len())))
            .spacing(Palette::SPACING)
            .align_items(iced::Alignment::Center);

        let error_banner = if self.errors.is_empty() {
            None
        } else {
            let mut list = Column::new().spacing(5);
            for (i, msg) in self.errors.iter().enumerate() {
                let row = row![
                    text(msg.clone()).size(16),
                    button("Dismiss")
                        .style(style::button_primary())
                        .on_press(Message::DismissError(i))
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center);
                list = list.push(row);
            }
            let banner = column![
                row![
                    text("Operation failed").size(16),
                    button("Dismiss All")
                        .style(style::button_primary())
                        .on_press(Message::ClearErrors)
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center),
                scrollable(list).height(Length::Fixed(100.0))
            ]
            .spacing(5);
            Some(container(banner).style(error_container_style()).padding(10).width(Length::Fill))
        };

        let album_dialog = album_dialogs::create_dialog(self);
        let rename_dialog = album_dialogs::rename_dialog(self);
        let delete_dialog = album_dialogs::delete_dialog(self);
        let settings_dialog = settings::dialog(self);

        let content = match &self.state {
            ViewState::Grid => {
                if self.loading {
                    column![header, text("Loading photos...").size(16),]
                } else if self.photos.is_empty() {
                    column![
                        header,
                        text("No photos found. Make sure you have authenticated and synced your photos.").size(16),
                    ]
                } else {
                    let mut album_row =
                        row![
                            button(text("All"))
                                .style(style::button_primary())
                                .on_press(Message::SelectAlbum(None))
                        ]
                        .spacing(10);
                    for album in &self.albums {
                        let title = album.title.clone().unwrap_or_else(|| "Untitled".to_string());
                        let controls = row![
                            button(text(title.clone()))
                                .style(style::button_primary())
                                .on_press(Message::SelectAlbum(Some(album.id.clone()))),
                            button(Icon::new(MaterialSymbol::Edit))
                                .style(style::button_primary())
                                .on_press(Message::ShowRenameAlbumDialog(album.id.clone(), title.clone())),
                            button(Icon::new(MaterialSymbol::Delete))
                                .style(style::button_secondary())
                                .on_press(Message::ShowDeleteAlbumDialog(album.id.clone()))
                        ]
                        .spacing(5);
                        album_row = album_row.push(controls);
                    }

                    let mut rows = column![].spacing(10);
                    let mut current = row![].spacing(10);
                    let mut count = 0;
                    for photo in self.photos.iter().take(self.display_limit) {
                        let thumb: Element<Message> =
                            if let Some(handle) = self.thumbnails.get(&photo.id) {
                                image(handle.clone())
                                    .width(Length::Fixed(150.0))
                                    .height(Length::Fixed(150.0))
                                    .into()
                            } else {
                                container(text("Loading..."))
                                    .width(Length::Fixed(150.0))
                                    .height(Length::Fixed(150.0))
                                    .into()
                            };
                        let btn = button(thumb)
                            .style(style::button_primary())
                            .on_press(Message::SelectPhoto(photo.clone()));
                        current = current.push(btn);
                        count += 1;
                        if count == 4 {
                            rows = rows.push(current);
                            current = row![].spacing(10);
                            count = 0;
                        }
                    }
                    if count > 0 {
                        rows = rows.push(current);
                    }
                    let mut grid = column![].spacing(10);
                    if self.display_limit < self.photos.len() {
                        grid = grid.push(
                            button("Load more")
                                .style(style::button_primary())
                                .on_press(Message::LoadMorePhotos),
                        );
                    }
                    column![
                        header,
                        scrollable(album_row).height(Length::Shrink),
                        text(format!("Found {} photos", self.photos.len())).size(16),
                        scrollable(rows).height(Length::Fill),
                        grid,
                    ]
                }
            }
            ViewState::SelectedPhoto { photo, faces } => {
                let img: Element<Message> = if let Some(handle) = self.full_images.get(&photo.id) {
                    let base = image(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill);
                    let w = photo.media_metadata.width.parse::<u32>().unwrap_or(0);
                    let h = photo.media_metadata.height.parse::<u32>().unwrap_or(0);
                    container(base)
                        .style(style::card())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .overlay(FaceRecognizer::new(faces.clone(), w, h).view())
                        .into()
                } else {
                    container(text("Loading..."))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                };
                let album_opts: Vec<AlbumOption> = self
                    .albums
                    .iter()
                    .map(|a| AlbumOption {
                        id: a.id.clone(),
                        title: a.title.clone().unwrap_or_else(|| "Untitled".into()),
                    })
                    .collect();
                let mut faces_col = column![];
                for (i, face) in faces.iter().enumerate() {
                    let row_elem = if self.editing_face == Some(i) {
                        row![
                            text_input("Name", &self.face_name_input)
                                .style(style::text_input())
                                .on_input(Message::FaceNameChanged),
                            button(Icon::new(MaterialSymbol::Save).color(Palette::ON_PRIMARY))
                                .style(style::button_primary())
                                .on_press(Message::SaveFaceName),
                            button(Icon::new(MaterialSymbol::Cancel).color(Palette::ON_SECONDARY))
                                .style(style::button_secondary())
                                .on_press(Message::CancelFaceName)
                        ]
                    } else {
                        let (x, y, w, h) = face.rect;
                        let label = format!(
                            "Face {} ({},{},{},{}): {}",
                            i + 1,
                            x,
                            y,
                            w,
                            h,
                            face.name.clone().unwrap_or_else(|| "Unknown".into())
                        );
                        row![
                            text(label),
                            button("Rename")
                                .style(style::button_primary())
                                .on_press(Message::StartRenameFace(i))
                        ]
                    };
                    faces_col = faces_col.push(row_elem);
                }
                let mut col = column![
                    header,
                    button(Icon::new(MaterialSymbol::Close).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::ClosePhoto),
                    img,
                    faces_col,
                    pick_list(
                        album_opts,
                        self.assign_selection.clone(),
                        Message::AlbumPicked
                    )
                ];
                #[cfg(feature = "gstreamer")]
                if photo.mime_type.starts_with("video/") {
                    col = col.push(
                        button(Icon::new(MaterialSymbol::PlayArrow).color(Palette::ON_PRIMARY))
                            .style(style::button_primary())
                            .on_press(Message::PlayVideo(photo.clone())),
                    );
                }
                #[cfg(not(feature = "gstreamer"))]
                if photo.mime_type.starts_with("video/") {
                    col = col.push(text("Video playback not available"));
                }
                col
            }
            #[cfg(feature = "gstreamer")]
            ViewState::PlayingVideo { player, .. } => {
                let frame = player
                    .frame_handle()
                    .unwrap_or_else(|| image::Handle::from_pixels(1, 1, vec![0, 0, 0, 0]));
                let duration = player.duration_seconds();
                let position = player.position_seconds();
                let play_icon = if matches!(player.play_status(), PlayStatus::Playing) {
                    MaterialSymbol::Pause
                } else {
                    MaterialSymbol::PlayArrow
                };
                column![
                    header,
                    row![
                        button(Icon::new(play_icon).color(Palette::ON_PRIMARY))
                            .style(style::button_primary())
                            .on_press(Message::ToggleVideoPlay),
                        slider(0.0..=duration, position, Message::SeekVideo)
                            .style(style::slider_primary())
                            .width(Length::Fill),
                        button(Icon::new(MaterialSymbol::Close).color(Palette::ON_PRIMARY))
                            .style(style::button_primary())
                            .on_press(Message::CloseVideo)
                    ]
                    .spacing(Palette::SPACING)
                    .align_items(iced::Alignment::Center),
                    image(frame).width(Length::Fill).height(Length::Fill)
                ]
            }
        };

        let mut base = column![].spacing(Palette::SPACING);
        if let Some(b) = error_banner {
            base = base.push(b);
        }
        base = base.push(content);
        if let Some(d) = album_dialog {
            base = base.push(d);
        }
        if let Some(d) = rename_dialog {
            base = base.push(d);
        }
        if let Some(d) = delete_dialog {
            base = base.push(d);
        }
        if let Some(d) = settings_dialog {
            base = base.push(d);
        }

        container(base)
            .style(style::card())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Palette::SPACING)
            .into()
    }
}
