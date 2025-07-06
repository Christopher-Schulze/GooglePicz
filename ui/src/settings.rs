use iced::widget::{button, checkbox, column, pick_list, row, text, text_input};

use crate::{style, Message};
use crate::style::Palette;

pub const LOG_LEVELS: [&str; 5] = ["trace", "debug", "info", "warn", "error"];

pub fn dialog<'a>(ui: &crate::GooglePiczUI) -> Option<iced::Element<'a, Message>> {
    if ui.settings_open {
        Some(
            column![
                text("Settings").size(16),
                pick_list(
                    &LOG_LEVELS[..],
                    Some(ui.settings_log_level.as_str()),
                    |v| Message::SettingsLogLevelChanged(v.to_string()),
                ),
                text_input("OAuth port", &ui.settings_oauth_port)
                    .style(style::text_input_basic())
                    .on_input(Message::SettingsOauthPortChanged),
                text_input("Thumbs preload", &ui.settings_thumbnails_preload)
                    .style(style::text_input_basic())
                    .on_input(Message::SettingsThumbsPreloadChanged),
                text_input("Preload threads", &ui.settings_preload_threads)
                    .style(style::text_input_basic())
                    .on_input(Message::SettingsPreloadThreadsChanged),
                text_input("Sync interval", &ui.settings_sync_interval)
                    .style(style::text_input_basic())
                    .on_input(Message::SettingsSyncIntervalChanged),
                checkbox(
                    "Debug console",
                    ui.settings_debug_console,
                    Message::SettingsDebugConsoleToggled,
                )
                .style(style::checkbox_primary()),
                checkbox(
                    "Trace spans",
                    ui.settings_trace_spans,
                    Message::SettingsTraceSpansToggled,
                )
                .style(style::checkbox_primary()),
                text_input("Cache path", &ui.settings_cache_path)
                    .style(style::text_input_basic())
                    .on_input(Message::SettingsCachePathChanged),
                row![
                    button("Save")
                        .style(style::button_primary())
                        .on_press(Message::SaveSettings),
                    button("Cancel")
                        .style(style::button_primary())
                        .on_press(Message::CloseSettings),
                ]
                .spacing(10),
            ]
            .spacing(10)
            .into(),
        )
    } else {
        None
    }
}

