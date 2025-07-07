use chrono::{DateTime, Utc};
use iced::widget::{button, checkbox, pick_list, row, text_input};

use crate::{style, Icon, MaterialSymbol, Message};
use crate::style::Palette;

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
    Faces,
}

impl SearchMode {
    pub const ALL: [SearchMode; 9] = [
        SearchMode::Filename,
        SearchMode::Description,
        SearchMode::Text,
        SearchMode::Favoriten,
        SearchMode::DateRange,
        SearchMode::MimeType,
        SearchMode::CameraModel,
        SearchMode::CameraMake,
        SearchMode::Faces,
    ];

    pub fn placeholder(self) -> &'static str {
        match self {
            SearchMode::Filename => "Filename",
            SearchMode::Description => "Description",
            SearchMode::Text => "Filename or description",
            SearchMode::Favoriten => "Favorites",
            SearchMode::MimeType => "Mime type",
            SearchMode::CameraModel => "Camera model",
            SearchMode::CameraMake => "Camera make",
            SearchMode::Faces => "Has faces",
            SearchMode::DateRange => "YYYY-MM-DD..YYYY-MM-DD",
        }
    }
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
            SearchMode::Faces => "Gesichter",
        };
        write!(f, "{}", s)
    }
}

pub(crate) fn parse_date_query(query: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
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

pub(crate) fn parse_single_date(query: &str, end: bool) -> Option<DateTime<Utc>> {
    use chrono::{NaiveDate, TimeZone};
    if let Ok(d) = NaiveDate::parse_from_str(query, "%Y-%m-%d") {
        let nd = if end { d.and_hms_opt(23, 59, 59)? } else { d.and_hms_opt(0, 0, 0)? };
        return Some(Utc.from_utc_datetime(&nd));
    }
    None
}

pub fn view<'a>(ui: &crate::GooglePiczUI) -> iced::Element<'a, Message> {
    row![
        text_input(ui.search_mode.placeholder(), &ui.search_query)
            .style(style::text_input())
            .on_input(Message::SearchInputChanged),
        text_input("Camera", &ui.search_camera)
            .style(style::text_input())
            .on_input(Message::SearchCameraChanged),
        text_input("From", &ui.search_start)
            .style(style::text_input())
            .on_input(Message::SearchStartChanged),
        text_input("To", &ui.search_end)
            .style(style::text_input())
            .on_input(Message::SearchEndChanged),
        checkbox("Fav", ui.search_favorite, Message::SearchFavoriteToggled)
            .style(style::checkbox_primary()),
        checkbox("Faces", ui.search_faces, Message::SearchFacesToggled)
            .style(style::checkbox_primary()),
        pick_list(&SearchMode::ALL[..], Some(ui.search_mode), Message::SearchModeChanged),
        button(Icon::new(MaterialSymbol::Search).color(Palette::ON_PRIMARY))
            .style(style::button_primary())
            .on_press(Message::PerformSearch)
    ]
    .spacing(Palette::SPACING)
    .align_items(iced::Alignment::Center)
    .into()
}

