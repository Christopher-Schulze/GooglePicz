//! User Interface module for GooglePicz.

mod image_loader;
#[path = "../app/src/config.rs"]
mod app_config;

pub use image_loader::{ImageLoader, ImageLoaderError};

use api_client::{Album, ApiClient, MediaItem};
use app_config::AppConfig;
use auth;
use cache::CacheManager;
use chrono::{DateTime, Utc};
use iced::subscription;
use iced::widget::container::Appearance;
use iced::widget::image::Handle;
use iced::widget::{
    button, checkbox, column, container, image, pick_list, row, scrollable, text,
    text_input, Column,
};
use iced::Border;
use iced::Color;
use iced::{executor, Application, Command, Element, Length, Settings, Subscription, Theme};
use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use sync::{SyncProgress, SyncTaskError};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
#[cfg(feature = "gstreamer")]
use gstreamer_iced::{GstreamerIcedBase, GStreamerMessage, PlayStatus};
#[cfg(feature = "gstreamer")]
use gstreamer_iced::reexport::url;

const ERROR_DISPLAY_DURATION: Duration = Duration::from_secs(5);

fn parse_date_query(query: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    use chrono::{NaiveDate, TimeZone};
    if let Some(idx) = query.find("..") {
        let start_str = &query[..idx];
        let end_str = &query[idx + 2..];
        if let (Ok(s), Ok(e)) = (
            NaiveDate::parse_from_str(start_str, "%Y-%m-%d"),
            NaiveDate::parse_from_str(end_str, "%Y-%m-%d"),
        ) {
            let start = Utc.from_utc_datetime(&s.and_hms_opt(0, 0, 0)?);
            let end = Utc.from_utc_datetime(&e.and_hms_opt(23, 59, 59)?);
            return Some((start, end));
        }
    } else if let Ok(d) = NaiveDate::parse_from_str(query, "%Y-%m-%d") {
        let start = Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?);
        let end = Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59)?);
        return Some((start, end));
    }
    None
}

fn parse_single_date(query: &str, end: bool) -> Option<DateTime<Utc>> {
    use chrono::{NaiveDate, TimeZone};
    if let Ok(d) = NaiveDate::parse_from_str(query, "%Y-%m-%d") {
        let nd = if end { d.and_hms_opt(23, 59, 59)? } else { d.and_hms_opt(0, 0, 0)? };
        return Some(Utc.from_utc_datetime(&nd));
    }
    None
}

fn error_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(|_theme: &Theme| Appearance {
        text_color: Some(Color::from_rgb(0.5, 0.0, 0.0)),
        background: Some(Color::from_rgb(1.0, 0.9, 0.9).into()),
        border: Border {
            color: Color::from_rgb(0.8, 0.0, 0.0),
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
    preload: usize,
    cache_dir: PathBuf,
) -> iced::Result {
    GooglePiczUI::run(Settings::with_flags((progress, errors, preload, cache_dir)))
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
    SelectPhoto(MediaItem),
    SelectAlbum(Option<String>),
    ClosePhoto,
    SyncProgress(SyncProgress),
    SyncError(SyncTaskError),
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
    ClearErrors,
    ShowSettings,
    CloseSettings,
    SettingsLogLevelChanged(String),
    SettingsCachePathChanged(String),
    SaveSettings,
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
    SelectedPhoto(MediaItem),
    #[cfg(feature = "gstreamer")]
    PlayingVideo(GstreamerIcedBase),
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
    synced: u64,
    syncing: bool,
    last_synced: Option<DateTime<Utc>>,
    sync_status: String,
    state: ViewState,
    selected_album: Option<String>,
    errors: Vec<String>,
    preload_count: usize,
    creating_album: bool,
    new_album_title: String,
    assign_selection: Option<AlbumOption>,
    renaming_album: Option<String>,
    rename_album_title: String,
    deleting_album: Option<String>,
    search_mode: SearchMode,
    search_query: String,
    search_camera: String,
    search_start: String,
    search_end: String,
    search_favorite: bool,
    error_log_path: PathBuf,
    settings_open: bool,
    config_path: PathBuf,
    settings_log_level: String,
    settings_cache_path: String,
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
    fn log_error(&self, msg: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.error_log_path)
        {
            let _ = writeln!(file, "{}", msg);
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
        usize,
        PathBuf,
    );

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(flags)))]
    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (progress_flag, error_flag, preload_count, cache_dir) = flags;
        let mut init_errors = Vec::new();
        let error_log_path = cache_dir.join("ui_errors.log");
        let cache_path = cache_dir.join("cache.sqlite");
        let config_path = cache_dir.join("config");

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
            cache.get_last_sync().ok()
        } else {
            None
        };

        let image_loader = Arc::new(Mutex::new(ImageLoader::new(cache_dir.clone())));

        let progress_receiver = progress_flag.map(|rx| Arc::new(Mutex::new(rx)));
        let error_receiver = error_flag.map(|rx| Arc::new(Mutex::new(rx)));

        let status = match last_synced {
            Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
            None => "Never synced".to_string(),
        };

        let cfg = AppConfig::load_from(Some(config_path.clone()));
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
            synced: 0,
            syncing: false,
            last_synced,
            sync_status: status,
            state: ViewState::Grid,
            selected_album: None,
            errors: init_errors,
            preload_count,
            creating_album: false,
            new_album_title: String::new(),
            assign_selection: None,
            renaming_album: None,
            rename_album_title: String::new(),
            deleting_album: None,
            search_mode: SearchMode::Filename,
            search_query: String::new(),
            search_camera: String::new(),
            search_start: String::new(),
            search_end: String::new(),
            search_favorite: false,
            error_log_path,
            settings_open: false,
            config_path,
            settings_log_level: cfg.log_level.clone(),
            settings_cache_path: cfg.cache_path.to_string_lossy().to_string(),
        };

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
                self.state = ViewState::SelectedPhoto(photo);
                return Command::perform(async {}, move |_| {
                    Message::LoadFullImage(id.clone(), url.clone())
                });
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
            Message::ClosePhoto => {
                self.state = ViewState::Grid;
            }
            #[cfg(feature = "gstreamer")]
            Message::PlayVideo(item) => {
                let url = format!("{}=dv", item.base_url);
                match GstreamerIcedBase::new_url(&url::Url::parse(&url).unwrap(), false) {
                    Ok(mut player) => {
                        let _ = player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Playing));
                        self.state = ViewState::PlayingVideo(player);
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
                        return GooglePiczUI::error_timeout();
                    }
                }
            }
            #[cfg(feature = "gstreamer")]
            Message::VideoEvent(msg) => {
                if let ViewState::PlayingVideo(player) = &mut self.state {
                    if let GStreamerMessage::BusGoToEnd = msg {
                        self.state = ViewState::Grid;
                        return Command::none();
                    }
                    return player.update(msg).map(Message::VideoEvent);
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
            Message::SyncError(err_msg) => {
                tracing::error!("Sync error: {}", err_msg);
                let detail = match &err_msg {
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
                self.errors.push(err_msg.to_string());
                self.log_error(&err_msg.to_string());
                self.sync_status = "Sync error".into();
                self.syncing = false;
                return GooglePiczUI::error_timeout();
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
            Message::SaveSettings => {
                let mut cfg = AppConfig::load_from(Some(self.config_path.clone()));
                cfg.log_level = self.settings_log_level.clone();
                cfg.cache_path = PathBuf::from(self.settings_cache_path.clone());
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
                if let ViewState::SelectedPhoto(photo) = &self.state {
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
            Message::SearchInputChanged(q) => {
                self.search_query = q;
            }
            Message::SearchModeChanged(mode) => {
                self.search_mode = mode;
            }
            Message::SearchCameraChanged(v) => {
                self.search_camera = v;
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
                    return Command::perform(
                        async move {
                            let cache = {
                                let guard = cm.lock().await;
                                guard.clone()
                            };
                            let base = match mode {
                                SearchMode::DateRange => {
                                    if let Some((s, e)) = parse_date_query(&query) {
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
                                SearchMode::Text => cache
                                    .get_media_items_by_text(&query)
                                    .map_err(|e| e.to_string()),
                                SearchMode::Favoriten => cache
                                    .get_media_items_by_favorite(true)
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
                            }?;

                            let start_dt = parse_single_date(&start, false);
                            let end_dt = parse_single_date(&end, true);
                            cache
                                .query_media_items_async(
                                    if camera.is_empty() { None } else { Some(camera) },
                                    start_dt,
                                    end_dt,
                                    if fav { Some(true) } else { None },
                                )
                                .await
                                .map_err(|e| e.to_string())
                                .map(|mut extra| {
                                    // Intersect results with base
                                    extra.retain(|i| base.iter().any(|b| b.id == i.id));
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
                    Some(e) => Message::SyncError(e),
                    None => Message::SyncProgress(SyncProgress::Finished(0)),
                };
                drop(lock);
                (msg, rx)
            }));
        }

        #[cfg(feature = "gstreamer")]
        if let ViewState::PlayingVideo(player) = &self.state {
            subs.push(player.subscription().map(Message::VideoEvent));
        }

        Subscription::batch(subs)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    fn view(&self) -> Element<Message> {
        let placeholder = match self.search_mode {
            SearchMode::MimeType => "Mime type",
            SearchMode::CameraModel => "Camera model",
            SearchMode::CameraMake => "Camera make",
            SearchMode::DateRange => "YYYY-MM-DD..YYYY-MM-DD",
            _ => "Search",
        };

        let mut header = row![
            text("GooglePicz").size(24),
            button("Refresh").on_press(Message::RefreshPhotos),
            button("New Album…").on_press(Message::ShowCreateAlbumDialog),
            button("Settings").on_press(Message::ShowSettings),
            text_input(placeholder, &self.search_query)
                .on_input(Message::SearchInputChanged),
            text_input("Camera", &self.search_camera).on_input(Message::SearchCameraChanged),
            text_input("From", &self.search_start).on_input(Message::SearchStartChanged),
            text_input("To", &self.search_end).on_input(Message::SearchEndChanged),
            checkbox("Fav", self.search_favorite, Message::SearchFavoriteToggled),
            pick_list(
                &SearchMode::ALL[..],
                Some(self.search_mode),
                Message::SearchModeChanged,
            ),
            button("Search").on_press(Message::PerformSearch)
        ];

        if let Some(album_id) = &self.selected_album {
            header = header
                .push(
                    button("Rename").on_press(Message::ShowRenameAlbumDialog(
                        album_id.clone(),
                        self.albums
                            .iter()
                            .find(|a| a.id == *album_id)
                            .and_then(|a| a.title.clone())
                            .unwrap_or_default(),
                    )),
                )
                .push(button("Delete").on_press(Message::ShowDeleteAlbumDialog(album_id.clone())));
        }

        header = header
            .push(text(self.sync_status.clone()))
            .push(text(match self.last_synced {
                Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
                None => "Never synced".to_string(),
            }))
            .push(text(format!("Errors: {}", self.errors.len())))
            .spacing(20)
            .align_items(iced::Alignment::Center);

        let error_banner = if self.errors.is_empty() {
            None
        } else {
            let mut banner = Column::new().spacing(5);
            banner = banner.push(
                row![
                    text("Operation failed").size(16),
                    button("Dismiss All").on_press(Message::ClearErrors)
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center),
            );
            for (i, msg) in self.errors.iter().enumerate() {
                let row = row![
                    text(msg.clone()).size(16),
                    button("Dismiss").on_press(Message::DismissError(i))
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center);
                banner = banner.push(row);
            }
            Some(container(banner).style(error_container_style()).padding(10).width(Length::Fill))
        };

        let album_dialog = if self.creating_album {
            Some(
                column![
                    text_input("Album title", &self.new_album_title)
                        .on_input(Message::AlbumTitleChanged),
                    row![
                        button("Create").on_press(Message::CreateAlbum),
                        button("Cancel").on_press(Message::CancelCreateAlbum)
                    ]
                    .spacing(10)
                ]
                .spacing(10),
            )
        } else {
            None
        };

        let rename_dialog = if let Some(_) = &self.renaming_album {
            Some(
                column![
                    text_input("New title", &self.rename_album_title)
                        .on_input(Message::RenameAlbumTitleChanged),
                    row![
                        button("Rename").on_press(Message::ConfirmRenameAlbum),
                        button("Cancel").on_press(Message::CancelRenameAlbum)
                    ]
                    .spacing(10)
                ]
                .spacing(10),
            )
        } else {
            None
        };

        let delete_dialog = if self.deleting_album.is_some() {
            Some(
                column![
                    text("Delete album?").size(16),
                    row![
                        button("Delete").on_press(Message::ConfirmDeleteAlbum),
                        button("Cancel").on_press(Message::CancelDeleteAlbum)
                    ]
                    .spacing(10)
                ]
                .spacing(10),
            )
        } else {
            None
        };

        let settings_dialog = if self.settings_open {
            Some(
                column![
                    text("Settings").size(16),
                    text_input("Log level", &self.settings_log_level)
                        .on_input(Message::SettingsLogLevelChanged),
                    text_input("Cache path", &self.settings_cache_path)
                        .on_input(Message::SettingsCachePathChanged),
                    row![
                        button("Save").on_press(Message::SaveSettings),
                        button("Cancel").on_press(Message::CloseSettings)
                    ]
                    .spacing(10)
                ]
                .spacing(10),
            )
        } else {
            None
        };

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
                        row![button(text("All")).on_press(Message::SelectAlbum(None))].spacing(10);
                    for album in &self.albums {
                        let title = album.title.clone().unwrap_or_else(|| "Untitled".to_string());
                        let controls = row![
                            button(text(title.clone())).on_press(Message::SelectAlbum(Some(album.id.clone()))),
                            button("Rename").on_press(Message::ShowRenameAlbumDialog(album.id.clone(), title.clone())),
                            button("Delete").on_press(Message::ShowDeleteAlbumDialog(album.id.clone()))
                        ]
                        .spacing(5);
                        album_row = album_row.push(controls);
                    }

                    let mut rows = column![].spacing(10);
                    let mut current = row![].spacing(10);
                    let mut count = 0;
                    for photo in &self.photos {
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
                        let btn = button(thumb).on_press(Message::SelectPhoto(photo.clone()));
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
                    column![
                        header,
                        scrollable(album_row).height(Length::Shrink),
                        text(format!("Found {} photos", self.photos.len())).size(16),
                        scrollable(rows).height(Length::Fill),
                    ]
                }
            }
            ViewState::SelectedPhoto(photo) => {
                let img: Element<Message> = if let Some(handle) = self.full_images.get(&photo.id) {
                    image(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
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
                let mut col = column![
                    header,
                    button("Close").on_press(Message::ClosePhoto),
                    img,
                    pick_list(
                        album_opts,
                        self.assign_selection.clone(),
                        Message::AlbumPicked
                    )
                ];
                #[cfg(feature = "gstreamer")]
                if photo.mime_type.starts_with("video/") {
                    col = col.push(button("Play Video").on_press(Message::PlayVideo(photo.clone())));
                }
                #[cfg(not(feature = "gstreamer"))]
                if photo.mime_type.starts_with("video/") {
                    col = col.push(text("Video playback not available"));
                }
                col
            }
            #[cfg(feature = "gstreamer")]
            ViewState::PlayingVideo(player) => {
                let frame = player
                    .frame_handle()
                    .unwrap_or_else(|| image::Handle::from_pixels(1, 1, vec![0, 0, 0, 0]));
                column![
                    header,
                    button("Close").on_press(Message::CloseVideo),
                    image(frame).width(Length::Fill).height(Length::Fill)
                ]
            }
        };

        let mut base = column![].spacing(20);
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
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}
