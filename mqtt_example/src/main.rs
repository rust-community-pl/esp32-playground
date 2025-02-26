mod config;
mod controls;
mod display;
mod event;
mod mqtt;
mod wifi;

use crate::controls::Controls;
use crate::display::{DisplayControls, QuizDisplay, QuizRenderer};
use crate::event::DeviceEvent;

use std::sync::mpsc;
use std::thread;

use embedded_svc::mqtt::client::QoS;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

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

    let mut pixel_buffer = [0_u8; 1024];
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

    let (sender, receiver) = mpsc::channel();
    let mut question_id = Default::default();
    let mut question_text = Default::default();
    let mut options: Vec<String> = Default::default();
    let mut selection: u8 = 0;

    thread::scope(|s| {
        let mqtt_receiver_thread =
            mqtt::spawn_receiver_thread(s, mqtt_connection, sender.clone()).unwrap();
        std::mem::forget(mqtt_receiver_thread);

        let controls_thread = controls.spawn_thread(s, sender.clone()).unwrap();
        std::mem::forget(controls_thread);

        mqtt::try_until_subscribed(&mut mqtt_client, "question");
        mqtt::try_until_subscribed(&mut mqtt_client, "off");

        loop {
            let event: DeviceEvent = receiver.recv().unwrap();
            match event {
                DeviceEvent::Off => {
                    display.off();
                }
                DeviceEvent::Question { data } => {
                    display.clear();
                    display.on();

                    let parts: Vec<_> = data.split('|').map(String::from).collect();
                    question_id = parts[0].clone();
                    question_text = parts[1].clone();
                    options = parts[2..6].to_vec();

                    display.draw_question(&question_text);
                    display.draw_options(&options, selection);
                }
                DeviceEvent::Select { data } => {
                    selection = data;
                    if question_id.is_empty() {
                        continue;
                    }
                    display.draw_options(&options, selection);
                }
                DeviceEvent::Enter { data } => {
                    if question_id.is_empty() {
                        continue;
                    }
                    let payload = format!("{}|{}|{}", device_id, question_id, data);
                    mqtt_client.enqueue("answer", QoS::AtLeastOnce, false, payload.as_bytes())?;

                    display.clear();
                    display.draw_answer_sent();

                    question_id.clear();
                }
            }
        }
    })
}
