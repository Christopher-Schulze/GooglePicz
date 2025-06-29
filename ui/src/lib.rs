//! User Interface module for GooglePicz.

mod image_loader;

use iced::widget::{button, column, container, text, scrollable, row, image};
use iced::widget::image::Handle;
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use cache::CacheManager;
use api_client::MediaItem;
use image_loader::ImageLoader;

pub fn run() -> iced::Result {
    GooglePiczUI::run(Settings::default())
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadPhotos,
    PhotosLoaded(Result<Vec<MediaItem>, String>),
    RefreshPhotos,
    ThumbnailLoaded(String, Result<Handle, String>),
    LoadThumbnail(String, String), // media_id, base_url
}

pub struct GooglePiczUI {
    photos: Vec<MediaItem>,
    loading: bool,
    cache_manager: Option<Arc<Mutex<CacheManager>>>,
    image_loader: Arc<Mutex<ImageLoader>>,
    thumbnails: std::collections::HashMap<String, Handle>,
    error: Option<String>,
}

impl Application for GooglePiczUI {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
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

        let app = Self {
            photos: Vec::new(),
            loading: false,
            cache_manager,
            image_loader,
            thumbnails: std::collections::HashMap::new(),
            error: None,
        };
        
        (app, Command::perform(async {}, |_| Message::LoadPhotos))
    }

    fn title(&self) -> String {
        String::from("GooglePicz - Google Photos Manager")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LoadPhotos => {
                if let Some(cache_manager) = &self.cache_manager {
                    self.loading = true;
                    self.error = None;
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
                        self.error = None;
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
                        eprintln!("Failed to load photos: {}", error);
                        self.error = Some(error);
                    }
                }
            }
            Message::RefreshPhotos => {
                self.error = None;
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
                        self.error = None;
                    }
                    Err(error) => {
                        eprintln!("Failed to load thumbnail for {}: {}", media_id, error);
                        self.error = Some(format!("Failed to load thumbnail: {}", error));
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let header = row![
            text("GooglePicz").size(24),
            button("Refresh").on_press(Message::RefreshPhotos),
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center);

        let mut base = column![header].spacing(20);

        if let Some(err) = &self.error {
            base = base.push(
                container(text(err)).style(iced::theme::Container::Box).padding(10),
            );
        }

        if self.loading {
            base = base.push(text("Loading photos...").size(16));
        } else if self.photos.is_empty() {
            base = base.push(
                text("No photos found. Make sure you have authenticated and synced your photos.").size(16),
            );
        } else {
            let photo_list = self.photos.iter().enumerate().fold(
                column![].spacing(10),
                |col, (index, photo)| {
                    let filename = &photo.filename;
                    let dimensions = format!("{}x{}",
                        photo.media_metadata.width,
                        photo.media_metadata.height
                    );
                    
                    let creation_time = &photo.media_metadata.creation_time;
                    
                    let photo_info = column![
                        text(format!("#{}: {}", index + 1, filename)).size(14),
                        text(format!("Dimensions: {}", dimensions)).size(12),
                        text(format!("Created: {}", creation_time)).size(12),
                    ]
                    .spacing(2);
                    
                    let thumbnail_view: Element<Message> = if let Some(handle) = self.thumbnails.get(&photo.id) {
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

                    col.push(row![thumbnail_view, container(photo_info).padding(10).style(iced::theme::Container::Box)].spacing(10))
                }
            );
            
            base = base
                .push(text(format!("Found {} photos", self.photos.len())).size(16))
                .push(scrollable(photo_list).height(Length::Fill));
        }

        container(base)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}