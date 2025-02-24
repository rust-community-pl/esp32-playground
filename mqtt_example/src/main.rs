mod config;
mod controls;
mod event;
mod mqtt;
mod wifi;

use crate::config::{CHAR_STYLE, CHAR_STYLE_SELECTED, DISPLAY_OFFSET, DISPLAY_SIZE, TEXTBOX_STYLE};
use crate::controls::Controls;

use std::thread;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;

use embedded_hal::spi::MODE_3;

use esp_idf_svc::hal::{
    delay::Ets,
    gpio::{AnyIOPin, PinDriver},
    peripherals::Peripherals,
    spi::{SpiConfig, SpiDeviceDriver, SpiDriverConfig},
    units::*,
};
use mipidsi::{interface::SpiInterface, models::ST7789, Builder};

use mipidsi::options::ColorInversion;

use embedded_svc::mqtt::client::QoS;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use crate::event::DeviceEvent;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable};
use embedded_text::TextBox;
use log::*;
use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let event_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let peripherals = Peripherals::take()?;

    let _wifi = wifi::configure(&event_loop, &nvs, peripherals.modem)?;
    std::mem::forget(_wifi);

    let (mut mqtt_client, mqtt_connection) = mqtt::configure()?;

    let controls = Controls::new(peripherals.pins.gpio0, peripherals.pins.gpio35)?;

    let mut backlight = PinDriver::output(peripherals.pins.gpio4)?;

    // TODO: move SPI/display configuration to a separate module
    // Configure SPI
    let config = SpiConfig::new().baudrate(26.MHz().into()).data_mode(MODE_3);
    let spi_device = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        Option::<AnyIOPin>::None,
        Some(peripherals.pins.gpio5),
        &SpiDriverConfig::new(),
        &config,
    )?;
    let mut buffer = [0_u8; 512];
    let spi_interface = SpiInterface::new(
        spi_device,
        PinDriver::output(peripherals.pins.gpio16)?,
        &mut buffer,
    );

    // Configure display
    let mut delay = Ets;
    let mut display = Builder::new(ST7789, spi_interface)
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(PinDriver::output(peripherals.pins.gpio23)?)
        .display_offset(DISPLAY_OFFSET.0, DISPLAY_OFFSET.1)
        .display_size(DISPLAY_SIZE.0, DISPLAY_SIZE.1)
        .init(&mut delay)
        .expect("Failed to init display");

    // Reset pixels
    display
        .clear(Rgb565::BLACK)
        .expect("Failed to clear display");

    let (sender, receiver) = mpsc::channel();
    let mut question_id = Default::default();
    let mut question_text = Default::default();
    let mut options: Vec<String> = Default::default();
    let mut selection: u8 = 0;

    let header_text_bounds = Rectangle::new(
        Point::zero(),
        Size::new(DISPLAY_SIZE.0.into(), (DISPLAY_SIZE.1 / 2).into()),
    );

    thread::scope(|s| {
        info!("[MQTT] Starting client");
        let _mqtt_receiver_thread =
            mqtt::spawn_receiver_thread(s, mqtt_connection, sender.clone()).unwrap();
        let _controls_thread = controls.spawn_thread(s, sender.clone()).unwrap();

        mqtt::try_until_subscribed(&mut mqtt_client, "question");
        mqtt::try_until_subscribed(&mut mqtt_client, "off");

        loop {
            let event: DeviceEvent = receiver.recv().unwrap();
            match event {
                DeviceEvent::Off => {
                    backlight.set_low()?;
                }
                DeviceEvent::Question { data } => {
                    display.clear(Rgb565::BLACK).unwrap();
                    backlight.set_high()?;
                    let parts: Vec<_> = data.split('|').map(String::from).collect();
                    question_id = parts[0].clone();
                    question_text = parts[1].clone();
                    options = parts[2..6].to_vec();
                    // TODO: duplicate, move to display module
                    let text_box = TextBox::with_textbox_style(
                        &question_text,
                        header_text_bounds,
                        CHAR_STYLE,
                        TEXTBOX_STYLE,
                    );
                    text_box.draw(&mut display).unwrap();
                    for (idx, option) in options.iter().enumerate() {
                        let selected = idx as u8 == selection;
                        Text::new(
                            format!("{} {}", if selected { ">" } else { " " }, option).as_str(),
                            Point::new(0, 20 * idx as i32 + DISPLAY_SIZE.1 as i32 / 2),
                            if selected {
                                CHAR_STYLE_SELECTED
                            } else {
                                CHAR_STYLE
                            },
                        )
                        .draw(&mut display)
                        .unwrap();
                    }
                }
                DeviceEvent::Select { data } => {
                    selection = data;
                    if question_id.is_empty() {
                        continue;
                    }
                    // TODO: duplicate, move to display module
                    Rectangle::new(
                        Point::new(0, (DISPLAY_SIZE.1 / 2 - 12).into()),
                        Size::new(10, 80),
                    )
                    .draw_styled(&PrimitiveStyle::with_fill(Rgb565::BLACK), &mut display)
                    .unwrap();
                    for (idx, option) in options.iter().enumerate() {
                        let selected = idx as u8 == selection;
                        Text::new(
                            format!("{} {}", if selected { ">" } else { " " }, option).as_str(),
                            Point::new(0, 20 * idx as i32 + DISPLAY_SIZE.1 as i32 / 2),
                            if selected {
                                CHAR_STYLE_SELECTED
                            } else {
                                CHAR_STYLE
                            },
                        )
                        .draw(&mut display)
                        .unwrap();
                    }
                }
                DeviceEvent::Enter { data } => {
                    if question_id.is_empty() {
                        continue;
                    }
                    // TODO: add device id (from MAC?)
                    let payload = format!("{}|{}", question_id, data);
                    mqtt_client.enqueue("answer", QoS::AtLeastOnce, false, payload.as_bytes())?;
                    // TODO: move to display module
                    display.clear(Rgb565::BLACK).unwrap();
                    let text_box = TextBox::with_textbox_style(
                        "Answer sent!",
                        header_text_bounds,
                        CHAR_STYLE,
                        TEXTBOX_STYLE,
                    );
                    text_box.draw(&mut display).unwrap();
                    question_id.clear();
                }
            }
        }
    })
}
