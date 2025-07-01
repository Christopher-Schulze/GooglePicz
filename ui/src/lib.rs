//! User Interface module for GooglePicz.

mod image_loader;

use api_client::{Album, ApiClient, MediaItem};
use auth;
use cache::CacheManager;
use chrono::{DateTime, Utc};
use iced::subscription;
use iced::widget::container::Appearance;
use iced::widget::image::Handle;
use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, text, text_input, Column,
};
use iced::Border;
use iced::Color;
use iced::{executor, Application, Command, Element, Length, Settings, Subscription, Theme};
use image_loader::ImageLoader;
use std::path::PathBuf;
use std::sync::Arc;
use sync::SyncProgress;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

const ERROR_DISPLAY_DURATION: Duration = Duration::from_secs(5);

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

pub fn run(
    progress: Option<mpsc::UnboundedReceiver<SyncProgress>>,
    errors: Option<mpsc::UnboundedReceiver<String>>,
    preload: usize,
) -> iced::Result {
    GooglePiczUI::run(Settings::with_flags((progress, errors, preload)))
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
    SyncError(String),
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
    ClearErrors,
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
    error_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<String>>>>,
    synced: u64,
    syncing: bool,
    last_synced: Option<DateTime<Utc>>,
    state: ViewState,
    selected_album: Option<String>,
    errors: Vec<String>,
    preload_count: usize,
    creating_album: bool,
    new_album_title: String,
    assign_selection: Option<AlbumOption>,
}

impl GooglePiczUI {
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
        Option<mpsc::UnboundedReceiver<String>>,
        usize,
    );

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (progress_flag, error_flag, preload_count) = flags;
        let mut init_errors = Vec::new();
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let cache_path = home_dir.join(".googlepicz").join("cache.sqlite");

        let cache_manager = match CacheManager::new(&cache_path) {
            Ok(cm) => Some(Arc::new(Mutex::new(cm))),
            Err(e) => {
                init_errors.push(format!("Failed to initialize cache: {}", e));
                None
            }
        };

        let last_synced = if let Some(cm) = &cache_manager {
            let cache = cm.blocking_lock();
            cache.get_last_sync().ok()
        } else {
            None
        };

        let thumbnail_cache_path = home_dir.join(".googlepicz").join("thumbnails");

        let image_loader = Arc::new(Mutex::new(ImageLoader::new(thumbnail_cache_path)));

        let progress_receiver = progress_flag.map(|rx| Arc::new(Mutex::new(rx)));
        let error_receiver = error_flag.map(|rx| Arc::new(Mutex::new(rx)));

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
            state: ViewState::Grid,
            selected_album: None,
            errors: init_errors,
            preload_count,
            creating_album: false,
            new_album_title: String::new(),
            assign_selection: None,
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
                            let cache = cache_manager.lock().await;
                            cache.get_all_media_items().map_err(|e| e.to_string())
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
                    self.errors.push(format!("Failed to load albums: {}", err));
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
                    self.errors.push(format!(
                        "Failed to load thumbnail for {}: {}",
                        media_id, error
                    ));
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
                    self.errors.push(format!("Failed to load image: {}", error));
                    return GooglePiczUI::error_timeout();
                }
            },
            Message::ClosePhoto => {
                self.state = ViewState::Grid;
            }
            Message::SyncProgress(progress) => match progress {
                SyncProgress::ItemSynced(count) => {
                    self.synced = count;
                    self.syncing = true;
                }
                SyncProgress::Finished(total) => {
                    self.synced = total;
                    self.syncing = false;
                    self.last_synced = Some(Utc::now());
                }
            },
            Message::SyncError(err_msg) => {
                self.errors.push(err_msg);
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
                            let cache = cm.lock().await;
                            if let Err(e) = cache.insert_album(&album) {
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
                    self.errors.push(format!("Failed to create album: {}", err));
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
                                let cache = cm.lock().await;
                                cache
                                    .associate_media_item_with_album(&media_id, &album_id)
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
                    self.errors.push(format!("Failed to assign photo: {}", e));
                    return GooglePiczUI::error_timeout();
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
                            let cache = cm.lock().await;
                            cache.rename_album(&id, &title).map_err(|e| e.to_string())?;
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
                            let cache = cm.lock().await;
                            cache.delete_album(&id).map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |_: Result<_, _>| Message::LoadAlbums,
                );
            }
        }
        Command::none()
    }

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

        Subscription::batch(subs)
    }

    fn view(&self) -> Element<Message> {
        let mut header = row![
            text("GooglePicz").size(24),
            button("Refresh").on_press(Message::RefreshPhotos),
            button("New Album…").on_press(Message::ShowCreateAlbumDialog)
        ];

        if let Some(album_id) = &self.selected_album {
            header = header
                .push(button("Rename").on_press(Message::RenameAlbum(album_id.clone(), "Renamed".into())))
                .push(button("Delete").on_press(Message::DeleteAlbum(album_id.clone())));
        }

        header = header
            .push(text(if self.syncing {
                format!("Syncing {} items...", self.synced)
            } else {
                format!("Synced {} items", self.synced)
            }))
            .push(text(match self.last_synced {
                Some(ts) => format!("Last synced {}", ts.to_rfc3339()),
                None => "Never synced".to_string(),
            }))
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
            error_column =
                error_column.push(container(row).style(error_container_style()).padding(10));
        }
        let error_banner = if self.errors.is_empty() {
            None
        } else {
            Some(error_column)
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
                            button("Rename").on_press(Message::RenameAlbum(album.id.clone(), format!("{}-renamed", title))),
                            button("Delete").on_press(Message::DeleteAlbum(album.id.clone()))
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
                column![
                    header,
                    button("Close").on_press(Message::ClosePhoto),
                    img,
                    pick_list(
                        album_opts,
                        self.assign_selection.clone(),
                        Message::AlbumPicked
                    )
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

        container(base)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}
