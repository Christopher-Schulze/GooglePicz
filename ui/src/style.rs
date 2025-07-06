//! Material design inspired styling for the UI.
//!
//! This module centralises all colors, spacing and basic widget styles.
//! New components should be built on top of these helpers so the
//! application keeps a consistent Material look.

use iced::{Color, Border};
use iced::widget::{
    self,
    button,
    checkbox,
    container,
    slider::{self, Handle, HandleShape, Rail},
    text_input,
};
use iced::theme;

/// Material color palette
pub struct Palette;

impl Palette {
    pub const PRIMARY: Color = Color { r: 0.25, g: 0.32, b: 0.71, a: 1.0 }; // Indigo 700
    pub const ON_PRIMARY: Color = Color::WHITE;
    pub const SURFACE: Color = Color { r: 0.98, g: 0.98, b: 0.98, a: 1.0 };
    pub const ON_SURFACE: Color = Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
    pub const ERROR: Color = Color { r: 0.80, g: 0.0, b: 0.0, a: 1.0 };

    pub const SPACING: u16 = 16;
    pub const ICON_COLOR: Color = Self::ON_SURFACE;
    pub const ICON_SIZE: u16 = 20;
}

/// Style for primary action buttons.
pub fn button_primary() -> theme::Button {
    theme::Button::Custom(Box::new(|_theme: &iced::Theme| button::Appearance {
        background: Some(Palette::PRIMARY.into()),
        border_radius: 4.0,
        text_color: Palette::ON_PRIMARY,
        ..Default::default()
    }))
}

/// Basic text input styling.
pub fn text_input_basic() -> theme::TextInput {
    theme::TextInput::Custom(Box::new(|_theme: &iced::Theme| text_input::Appearance {
        background: Palette::SURFACE.into(),
        border_radius: 4.0,
        border_width: 1.0,
        border_color: Palette::PRIMARY,
        icon_color: Palette::ON_SURFACE,
        placeholder_color: Palette::ON_SURFACE,
        value_color: Palette::ON_SURFACE,
        selection_color: Palette::PRIMARY,
    }))
}

/// Container style that mimics Material "cards".
pub fn card() -> theme::Container {
    theme::Container::Custom(Box::new(|_theme: &iced::Theme| container::Appearance {
        background: Some(Palette::SURFACE.into()),
        text_color: Some(Palette::ON_SURFACE),
        border: Border {
            color: Palette::PRIMARY,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Default::default(),
    }))
}

/// Style for checkboxes using the primary color.
pub fn checkbox_primary() -> theme::Checkbox {
    theme::Checkbox::Custom(Box::new(|_theme: &iced::Theme, is_checked: bool| {
        checkbox::Appearance {
            background: if is_checked {
                Palette::PRIMARY.into()
            } else {
                Palette::SURFACE.into()
            },
            icon_color: Palette::ON_PRIMARY,
            border: Border {
                color: Palette::PRIMARY,
                width: 1.0,
                radius: 2.0.into(),
            },
            text_color: None,
        }
    }))
}

/// Style for sliders matching the primary color palette.
pub fn slider_primary() -> theme::Slider {
    theme::Slider::Custom(Box::new(|_theme: &iced::Theme| slider::Appearance {
        rail: slider::Rail {
            colors: (Palette::PRIMARY, Palette::SURFACE),
            width: 4.0,
            border_radius: 2.0.into(),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            color: Palette::PRIMARY,
            border_width: 1.0,
            border_color: Palette::ON_PRIMARY,
        },
    }))
}

/// Example on how to create additional styled components:
///
/// ```ignore
/// use crate::style::{Palette};
/// use iced::widget::{checkbox, Checkbox};
/// use iced::theme;
///
/// pub fn checkbox_primary() -> theme::Checkbox {
///     theme::Checkbox::Custom(Box::new(|_theme: &iced::Theme, is_checked: bool| {
///         checkbox::Appearance {
///             background: Palette::SURFACE.into(),
///             checkmark_color: if is_checked { Palette::PRIMARY } else { Palette::ON_SURFACE },
///             border_radius: 2.0,
///             border_width: 1.0,
///             border_color: Palette::PRIMARY,
///         }
///     }))
/// }
/// ```
///
/// Custom components should use these helpers to maintain a consistent style.
