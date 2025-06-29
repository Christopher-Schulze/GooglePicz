//! User Interface module for GooglePicz.

mod image_loader;

use iced::widget::{button, column, container, text, scrollable, row, image};
use iced::widget::image::Handle;
use iced::{executor, Application, Command, Element, Length, Settings, Theme, Subscription};
use iced::subscription;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use cache::CacheManager;
use api_client::MediaItem;
use image_loader::ImageLoader;
use tokio::sync::mpsc;
use sync::SyncProgress;

pub fn run(progress: Option<mpsc::UnboundedReceiver<SyncProgress>>) -> iced::Result {
    GooglePiczUI::run(Settings::with_flags(progress))
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadPhotos,
    PhotosLoaded(Result<Vec<MediaItem>, String>),
    RefreshPhotos,
    ThumbnailLoaded(String, Result<Handle, String>),
    LoadThumbnail(String, String), // media_id, base_url
    LoadFullImage(String, String),
    FullImageLoaded(String, Result<Handle, String>),
    SelectPhoto(MediaItem),
    ClosePhoto,
    SyncProgress(SyncProgress),
    LastSyncLoaded(Result<Option<String>, String>),
}

#[derive(Debug, Clone)]
enum ViewState {
    Grid,
    SelectedPhoto(MediaItem),
}

pub struct GooglePiczUI {
    photos: Vec<MediaItem>,
    loading: bool,
    cache_manager: Option<Arc<Mutex<CacheManager>>>,
    image_loader: Arc<Mutex<ImageLoader>>,
    thumbnails: std::collections::HashMap<String, Handle>,
    full_images: std::collections::HashMap<String, Handle>,
    progress_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<SyncProgress>>>>,
    synced: u64,
    syncing: bool,
    last_sync: Option<String>,
    state: ViewState,
    errors: Vec<String>,
}

impl Application for GooglePiczUI {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Option<mpsc::UnboundedReceiver<SyncProgress>>;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let cache_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("cache.sqlite");

        let cache_manager = if let Ok(cm) = CacheManager::new(&cache_path) {
            Some(Arc::new(Mutex::new(cm)))
        } else {
            None
        };

        let thumbnail_cache_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("thumbnails");

        let image_loader = Arc::new(Mutex::new(ImageLoader::new(thumbnail_cache_path)));

        let progress_receiver = flags.map(|rx| Arc::new(Mutex::new(rx)));

        let last_sync = None;

        let app = Self {
            photos: Vec::new(),
            loading: false,
            cache_manager,
            image_loader,
            thumbnails: std::collections::HashMap::new(),
            full_images: std::collections::HashMap::new(),
            progress_receiver,
            synced: 0,
            syncing: false,
            last_sync,
            state: ViewState::Grid,
            errors: Vec::new(),
        };
        
        let cmd_load = Command::perform(async {}, |_| Message::LoadPhotos);
        let cmd_sync_time = if let Some(cm) = &app.cache_manager {
            let cm = cm.clone();
            Command::perform(async move { let cache = cm.lock().await; cache.get_last_sync().map_err(|e| e.to_string()) }, Message::LastSyncLoaded)
        } else { Command::none() };
        (app, Command::batch(vec![cmd_load, cmd_sync_time]))
    }

    fn title(&self) -> String {
        String::from("GooglePicz - Google Photos Manager")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LoadPhotos => {
                if let Some(cache_manager) = &self.cache_manager {
                    self.loading = true;
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
                        // Start loading thumbnails for all photos
                        let mut commands = Vec::new();
                        for photo in &self.photos {
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
            Message::RefreshPhotos => {
                return Command::perform(async {}, |_| Message::LoadPhotos);
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
                        if let Some(cm) = &self.cache_manager {
                            let cm = cm.clone();
                            return Command::perform(
                                async move {
                                    let cache = cm.lock().await;
                                    cache.get_last_sync().map_err(|e| e.to_string())
                                },
                                Message::LastSyncLoaded,
                            );
                        }
                    }
                }
            }
            Message::LastSyncLoaded(res) => {
                if let Ok(ts) = res {
                    self.last_sync = ts;
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
            text(match &self.last_sync {
                Some(ts) => format!("Last synced {}", ts),
                None => String::from("Never synced"),
            })
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center);

        let error_banner = self.errors.last().map(|msg| {
            container(text(msg)).style(iced::theme::Container::Box).padding(10)
        });

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