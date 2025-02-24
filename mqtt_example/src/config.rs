use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyle, TextBoxStyleBuilder};

pub const WIFI_SSID: &str = env!("WIFI_SSID");
pub const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

pub const MQTT_BROKER_URL: &str = env!("MQTT_BROKER_URL");
pub const MQTT_USER: &str = env!("MQTT_USER");
pub const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");

pub const DISPLAY_OFFSET: (u16, u16) = (52, 40);
pub const DISPLAY_SIZE: (u16, u16) = (135, 240);

pub const CHAR_STYLE: MonoTextStyle<Rgb565> = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
pub const CHAR_STYLE_SELECTED: MonoTextStyle<Rgb565> =
    MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
pub const TEXTBOX_STYLE: TextBoxStyle = TextBoxStyleBuilder::new()
    .height_mode(HeightMode::FitToText)
    .alignment(HorizontalAlignment::Left)
    .build();
