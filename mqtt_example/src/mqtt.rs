use embedded_svc::mqtt::client::{EventPayload, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration};
use esp_idf_svc::sys::esp_crt_bundle_attach;
use log::*;
use std::sync::mpsc;
use std::thread;
use std::thread::{sleep, Scope, ScopedJoinHandle};
use std::time::Duration;

use crate::config::{MQTT_BROKER_URL, MQTT_PASSWORD, MQTT_USER};
use crate::event::DeviceEvent;

pub fn configure() -> anyhow::Result<(EspMqttClient<'static>, EspMqttConnection)> {
    let mqtt_config = MqttClientConfiguration {
        username: Some(MQTT_USER),
        password: Some(MQTT_PASSWORD),
        // Enable MQTTS (MQTT with TLS)
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        keep_alive_interval: Some(Duration::from_secs(10)),
        reconnect_timeout: Some(Duration::from_secs(10)),
        network_timeout: Duration::from_secs(10),
        ..Default::default()
    };

    let (mqtt_client, mqtt_connection) = EspMqttClient::new(MQTT_BROKER_URL, &mqtt_config)?;

    Ok((mqtt_client, mqtt_connection))
}

/// Spawns a thread, that receives mqtt messages, parses them and sends to mpsc channel.
pub fn spawn_receiver_thread<'scope>(
    scope: &'scope Scope<'scope, '_>,
    mut mqtt_connection: EspMqttConnection,
    sender: mpsc::Sender<DeviceEvent>,
) -> Result<ScopedJoinHandle<'scope, ()>, std::io::Error> {
    thread::Builder::new()
        .stack_size(8192)
        .spawn_scoped(scope, move || loop {
            info!("[MQTT] Listening for messages");
            while let Ok(event) = mqtt_connection.next() {
                let payload = event.payload();
                debug!("[MQTT] {}", payload);
                if let EventPayload::Received {
                    id: _,
                    topic,
                    data,
                    details: _,
                } = payload
                {
                    sender
                        .send(DeviceEvent::from_mqtt_payload(topic.unwrap(), data).unwrap())
                        .ok();
                    sleep(Duration::from_millis(500))
                }
            }
            error!("[MQTT] Connection closed");
        })
}

/// Retries subscribing to topic until mqtt connection is established and subscription succeeds.
pub fn try_until_subscribed(mqtt_client: &mut EspMqttClient, topic: &str) {
    loop {
        if let Err(_e) = mqtt_client.subscribe(topic, QoS::ExactlyOnce) {
            sleep(Duration::from_millis(500));
            continue;
        }
        debug!("[MQTT] Subscribed to {topic}");
        break;
    }
}
