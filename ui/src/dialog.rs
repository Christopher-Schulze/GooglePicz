use iced::widget::container::Appearance;
use iced::{Border, Color, Theme};

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumOption {
    pub(crate) id: String,
    pub(crate) title: String,
}

impl std::fmt::Display for AlbumOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

pub fn error_container_style() -> iced::theme::Container {
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
