//! User Interface module for GooglePicz.

mod image_loader;

use iced::widget::{
    button, column, container, text, scrollable, row, image, Column
};
use iced::widget::image::Handle;
use iced::{executor, Application, Command, Element, Length, Settings, Theme, Subscription};
use iced::subscription;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use cache::CacheManager;
use api_client::{MediaItem, Album, ApiClient};
use auth;
use image_loader::ImageLoader;
use tokio::sync::mpsc;
use sync::SyncProgress;
use chrono::{DateTime, Utc};

pub fn run(progress: Option<mpsc::UnboundedReceiver<SyncProgress>>, preload: usize) -> iced::Result {
    GooglePiczUI::run(Settings::with_flags((progress, preload)))
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
    DismissError(usize),
}

#[derive(Debug, Clone)]
enum ViewState {
    Grid,
    SelectedPhoto(MediaItem),
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
    synced: u64,
    syncing: bool,
    last_synced: Option<DateTime<Utc>>,
    state: ViewState,
    selected_album: Option<String>,
    errors: Vec<String>,
    preload_count: usize,
}

impl Application for GooglePiczUI {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = (Option<mpsc::UnboundedReceiver<SyncProgress>>, usize);

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (progress_flag, preload_count) = flags;
        let cache_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("cache.sqlite");

        let cache_manager = if let Ok(cm) = CacheManager::new(&cache_path) {
            Some(Arc::new(Mutex::new(cm)))
        } else {
            None
        };

        let last_synced = if let Some(cm) = &cache_manager {
            let cache = cm.blocking_lock();
            cache.get_last_sync().ok()
        } else { None };

        let thumbnail_cache_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("thumbnails");

        let image_loader = Arc::new(Mutex::new(ImageLoader::new(thumbnail_cache_path)));

        let progress_receiver = progress_flag.map(|rx| Arc::new(Mutex::new(rx)));

        let app = Self {
            photos: Vec::new(),
            albums: Vec::new(),
            loading: false,
            cache_manager,
            image_loader,
            thumbnails: std::collections::HashMap::new(),
            full_images: std::collections::HashMap::new(),
            progress_receiver,
            synced: 0,
            syncing: false,
            last_synced,
            state: ViewState::Grid,
            selected_album: None,
            errors: Vec::new(),
            preload_count,
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LoadPhotos => {
                self.loading = true;
                if let Some(album_id) = &self.selected_album {
                    let album_id = album_id.clone();
                    return Command::perform(async move {
                        let token = auth::ensure_access_token_valid().await.map_err(|e| e.to_string())?;
                        let client = ApiClient::new(token);
                        client.get_album_media_items(&album_id, 100, None).await
                            .map(|r| r.0)
                            .map_err(|e| e.to_string())
                    }, Message::PhotosLoaded);
                } else if let Some(cache_manager) = &self.cache_manager {
                    let cache_manager = cache_manager.clone();
                    return Command::perform(
                        async move {
                            let cache = cache_manager.lock().await;
                            cache.get_all_media_items()
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
                            commands.push(Command::perform(async {}, move |_| Message::LoadThumbnail(media_id.clone(), base_url.clone())));
                        }
                        return Command::batch(commands);
                    }
                    Err(error) => {
                        self.errors.push(format!("Failed to load photos: {}", error));
                    }
                }
            }
            Message::LoadAlbums => {
                return Command::perform(async {
                    let token = auth::ensure_access_token_valid().await.map_err(|e| e.to_string())?;
                    let client = ApiClient::new(token);
                    client.list_albums(50, None).await
                        .map(|r| r.0)
                        .map_err(|e| e.to_string())
                }, Message::AlbumsLoaded);
            }
            Message::AlbumsLoaded(result) => {
                match result {
                    Ok(albums) => { self.albums = albums; }
                    Err(err) => { self.errors.push(format!("Failed to load albums: {}", err)); }
                }
            }
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
                    move |result| Message::ThumbnailLoaded(media_id, result.map_err(|e| e.to_string())),
                );
            }
            Message::ThumbnailLoaded(media_id, result) => {
                match result {
                    Ok(handle) => {
                        self.thumbnails.insert(media_id, handle);
                    }
                    Err(error) => {
                        self.errors.push(format!(
                            "Failed to load thumbnail for {}: {}",
                            media_id, error
                        ));
                    }
                }
            }
            Message::SelectPhoto(photo) => {
                let id = photo.id.clone();
                let url = photo.base_url.clone();
                self.state = ViewState::SelectedPhoto(photo);
                return Command::perform(async {}, move |_| Message::LoadFullImage(id.clone(), url.clone()));
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
            Message::FullImageLoaded(media_id, result) => {
                match result {
                    Ok(handle) => {
                        self.full_images.insert(media_id, handle);
                    }
                    Err(error) => {
                        self.errors.push(format!("Failed to load image: {}", error));
                    }
                }
            }
            Message::ClosePhoto => {
                self.state = ViewState::Grid;
            }
            Message::SyncProgress(progress) => {
                match progress {
                    SyncProgress::ItemSynced(count) => {
                        self.synced = count;
                        self.syncing = true;
                    }
                    SyncProgress::Finished(total) => {
                        self.synced = total;
                        self.syncing = false;
                        self.last_synced = Some(Utc::now());
                    }
                }
            }
            Message::DismissError(index) => {
                if index < self.errors.len() {
                    self.errors.remove(index);
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Some(progress_rx) = &self.progress_receiver {
            let progress_rx = progress_rx.clone();
            subscription::unfold("progress", progress_rx, |rx| async move {
                let mut lock = rx.lock().await;
                let msg = match lock.recv().await {
                    Some(p) => Message::SyncProgress(p),
                    None => Message::SyncProgress(SyncProgress::Finished(0)),
                };
                drop(lock);
                (msg, rx)
            })
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        let header = row![
            text("GooglePicz").size(24),
            button("Refresh").on_press(Message::RefreshPhotos),
            text(if self.syncing {
                format!("Syncing {} items...", self.synced)
            } else {
                format!("Synced {} items", self.synced)
            }),
            text(
                match self.last_synced {
                    Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
                    None => "Never synced".to_string(),
                }
            )
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center);

        let mut error_column = Column::new().spacing(5);
        for (i, msg) in self.errors.iter().enumerate() {
            let row = row![
                text(msg.clone()).size(16),
                button("Dismiss").on_press(Message::DismissError(i))
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center);
            error_column = error_column.push(
                container(row)
                    .style(iced::theme::Container::Box)
                    .padding(10),
            );
        }
        let error_banner = if self.errors.is_empty() {
            None
        } else {
            Some(error_column)
        };

        let content = match &self.state {
            ViewState::Grid => {
                if self.loading {
                    column![
                        header,
                        text("Loading photos...").size(16),
                    ]
                } else if self.photos.is_empty() {
                    column![
                        header,
                        text("No photos found. Make sure you have authenticated and synced your photos.").size(16),
                    ]
                } else {
                    let mut album_row = row![button(text("All")).on_press(Message::SelectAlbum(None))].spacing(10);
                    for album in &self.albums {
                        let title = album.title.clone().unwrap_or_else(|| "Untitled".to_string());
                        album_row = album_row.push(button(text(title)).on_press(Message::SelectAlbum(Some(album.id.clone()))));
                    }

                    let mut rows = column![].spacing(10);
                    let mut current = row![].spacing(10);
                    let mut count = 0;
                    for photo in &self.photos {
                        let thumb: Element<Message> = if let Some(handle) = self.thumbnails.get(&photo.id) {
                            image(handle.clone())
                                .width(Length::Fixed(150.0))
                                .height(Length::Fixed(150.0))
                                .into()
                        } else {
                            container(text("Loading...") )
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
                    image(handle.clone()).width(Length::Fill).height(Length::Fill).into()
                } else {
                    container(text("Loading...")).width(Length::Fill).height(Length::Fill).into()
                };
                column![
                    header,
                    button("Close").on_press(Message::ClosePhoto),
                    img
                ]
            }
        };

        let mut base = column![].spacing(20);
        if let Some(b) = error_banner { base = base.push(b); }
        base = base.push(content);

        container(base)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}
