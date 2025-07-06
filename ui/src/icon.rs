use iced::{Element, Color};
use iced::theme;
use google_material_symbols::{IcedExt, GoogleMaterialSymbols};
use crate::style::Palette;

/// Alias type for convenience
pub type MaterialSymbol = GoogleMaterialSymbols;

/// Simple icon wrapper using Material Symbols font
#[derive(Debug, Clone, Copy)]
pub struct Icon {
    symbol: MaterialSymbol,
    size: u16,
    color: Color,
}

impl Icon {
    /// Create a new icon with default size and color
    pub fn new(symbol: MaterialSymbol) -> Self {
        Self {
            symbol,
            size: Palette::ICON_SIZE,
            color: Palette::ICON_COLOR,
        }
    }

    /// Change icon size
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    /// Change icon color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'a, Message> From<Icon> for Element<'a, Message> {
    fn from(icon: Icon) -> Self {
        icon
            .symbol
            .into_text(icon.size)
            .style(theme::Text::Color(icon.color))
            .into()
    }
}
