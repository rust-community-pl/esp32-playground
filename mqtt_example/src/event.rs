pub enum DeviceEvent {
    // MQTT events
    Sleep,
    Question { data: Box<str> },
    Winner { data: Box<str> },
    // Button events
    Select { data: u8 },
    Enter { data: u8 },
    // Battery reader
    BatteryLevel { data: Option<u8> },
}

impl DeviceEvent {
    pub fn from_mqtt_payload(topic: &str, data: &[u8]) -> Option<Self> {
        match topic {
            "sleep" => Some(DeviceEvent::Sleep),
            "question" => Some(DeviceEvent::Question {
                data: String::from_utf8_lossy(data).into_owned().into_boxed_str(),
            }),
            "winner" => Some(DeviceEvent::Winner {
                data: String::from_utf8_lossy(data).into_owned().into_boxed_str(),
            }),
            _ => None,
        }
    }
}
