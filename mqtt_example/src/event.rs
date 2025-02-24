pub enum DeviceEvent {
    // MQTT events
    Off,
    Question { data: Box<str> },
    // Button events
    Select { data: u8 },
    Enter { data: u8 },
}

impl DeviceEvent {
    pub fn from_mqtt_payload(topic: &str, data: &[u8]) -> Option<Self> {
        match topic {
            "off" => Some(DeviceEvent::Off),
            "question" => Some(DeviceEvent::Question {
                data: String::from_utf8_lossy(data).into_owned().into_boxed_str(),
            }),
            _ => None,
        }
    }
}
