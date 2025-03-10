mod battery;
mod config;
mod controls;
mod display;
mod event;
mod mqtt;
mod wifi;

use crate::controls::Controls;
use crate::display::{DisplayControls, QuizDisplay, QuizRenderer};
use crate::event::DeviceEvent;

use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let event_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let peripherals = Peripherals::take()?;

    let mut wifi = wifi::configure(&event_loop, &nvs, peripherals.modem)?;
    let device_id = wifi::get_mac(&mut wifi);
    std::mem::forget(wifi);

    let (mut mqtt_client, mqtt_connection) = mqtt::configure()?;

    let controls = Controls::new(peripherals.pins.gpio0, peripherals.pins.gpio35)?;

    let mut pixel_buffer = [0_u8; 2048];
    let mut display = QuizDisplay::new(
        peripherals.spi2,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        peripherals.pins.gpio5,
        peripherals.pins.gpio16,
        peripherals.pins.gpio23,
        peripherals.pins.gpio4,
        &mut pixel_buffer,
    );
    display.clear();
    display.off();

    let (sender, receiver) = mpsc::channel();
    let mut question_id = String::new();
    let mut question_text = String::new();
    let mut options: Vec<String> = Default::default();
    let mut selection: u8 = 0;
    let mut battery_level: Option<u8> = Some(0);

    thread::scope(|s| {
        controls.spawn_thread(s, sender.clone()).unwrap();
        mqtt::spawn_receiver_thread(s, mqtt_connection, sender.clone()).unwrap();
        battery::spawn_reader_thread(s, peripherals.adc1, peripherals.pins.gpio34, sender.clone())
            .unwrap();

        mqtt::try_until_subscribed(&mut mqtt_client, "question");
        mqtt::try_until_subscribed(&mut mqtt_client, "sleep");
        mqtt::try_until_subscribed(&mut mqtt_client, "winner");
        mqtt::try_until_subscribed(&mut mqtt_client, "message");

        loop {
            let event: DeviceEvent = receiver.recv().unwrap();
            match event {
                DeviceEvent::Sleep => {
                    display.clear();
                    display.draw_battery_level(battery_level);
                    display.off();
                    question_id.clear();
                }
                DeviceEvent::Question { data } => {
                    display.clear();
                    display.draw_battery_level(battery_level);
                    display.on();

                    let parts: Vec<_> = data.split('|').map(String::from).collect();
                    question_id = parts[0].clone();
                    question_text = parts[1].clone();
                    options = parts[2..6].to_vec();

                    display.draw_question(&question_text);
                    display.draw_options(&options, selection);
                }
                DeviceEvent::Winner { data } => {
                    if data.into_string() == device_id {
                        display.clear();
                        display.draw_battery_level(battery_level);
                        display.on();
                        display.draw_text(&format!("You won!\n{}", device_id));
                    }
                }
                DeviceEvent::Message { data } => {
                    display.clear();
                    display.draw_battery_level(battery_level);
                    display.draw_text(&String::from(data));
                    display.on();
                }
                DeviceEvent::Select { data } => {
                    selection = data;
                    if question_id.is_empty() {
                        display.on();
                        thread::sleep(Duration::from_millis(1000));
                        display.off();
                        continue;
                    }
                    display.draw_options(&options, selection);
                }
                DeviceEvent::Enter { data } => {
                    if question_id.is_empty() {
                        display.on();
                        thread::sleep(Duration::from_millis(1000));
                        display.off();
                        continue;
                    }
                    let payload = format!("{}|{}|{}", device_id, question_id, data);
                    mqtt_client.enqueue("answer", QoS::AtLeastOnce, false, payload.as_bytes())?;

                    display.clear();
                    display.draw_battery_level(battery_level);
                    display.draw_text("Answer sent!");

                    question_id.clear();
                }
                DeviceEvent::BatteryLevel { data } => {
                    battery_level = data;
                    display.draw_battery_level(battery_level);
                }
            }
        }
    })
}
