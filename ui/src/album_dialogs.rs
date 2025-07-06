use iced::widget::{button, column, row, text, text_input};

use crate::{style, Icon, MaterialSymbol, Message};
use crate::style::Palette;

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumOption {
    pub id: String,
    pub title: String,
}

impl std::fmt::Display for AlbumOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

pub fn create_dialog<'a>(ui: &crate::GooglePiczUI) -> Option<iced::Element<'a, Message>> {
    if ui.creating_album {
        Some(
            container(
                column![
                text_input("Album title", &ui.new_album_title)
                    .style(style::text_input())
                    .on_input(Message::AlbumTitleChanged),
                row![
                    button(Icon::new(MaterialSymbol::Add).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::CreateAlbum),
                    button(Icon::new(MaterialSymbol::Cancel).color(Palette::ON_SECONDARY))
                        .style(style::button_secondary())
                        .on_press(Message::CancelCreateAlbum),
                ]
                .spacing(Palette::SPACING),
            ]
            .spacing(Palette::SPACING))
                .style(style::dialog())
                .padding(Palette::SPACING)
                .into(),
        )
    } else {
        None
    }
}

pub fn rename_dialog<'a>(ui: &crate::GooglePiczUI) -> Option<iced::Element<'a, Message>> {
    if ui.renaming_album.is_some() {
        Some(
            container(
                column![
                text_input("New title", &ui.rename_album_title)
                    .style(style::text_input())
                    .on_input(Message::RenameAlbumTitleChanged),
                row![
                    button(Icon::new(MaterialSymbol::Save).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::ConfirmRenameAlbum),
                    button(Icon::new(MaterialSymbol::Cancel).color(Palette::ON_SECONDARY))
                        .style(style::button_secondary())
                        .on_press(Message::CancelRenameAlbum),
                ]
                .spacing(Palette::SPACING),
            ]
            .spacing(Palette::SPACING))
                .style(style::dialog())
                .padding(Palette::SPACING)
                .into(),
        )
    } else {
        None
    }
}

pub fn delete_dialog<'a>(ui: &crate::GooglePiczUI) -> Option<iced::Element<'a, Message>> {
    if ui.deleting_album.is_some() {
        Some(
            container(
                column![
                text("Delete album?").size(16),
                row![
                    button(Icon::new(MaterialSymbol::Delete).color(Palette::ON_PRIMARY))
                        .style(style::button_primary())
                        .on_press(Message::ConfirmDeleteAlbum),
                    button(Icon::new(MaterialSymbol::Cancel).color(Palette::ON_SECONDARY))
                        .style(style::button_secondary())
                        .on_press(Message::CancelDeleteAlbum),
                ]
                .spacing(Palette::SPACING),
            ]
            .spacing(Palette::SPACING))
                .style(style::dialog())
                .padding(Palette::SPACING)
                .into(),
        )
    } else {
        None
    }
}

