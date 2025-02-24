use crate::config::{WIFI_PASSWORD, WIFI_SSID};

use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::info;

pub fn configure(
    event_loop: &EspSystemEventLoop,
    nvs: &EspDefaultNvsPartition,
    modem: Modem,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, event_loop.clone(), Some(nvs.clone()))?,
        event_loop.clone(),
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().expect("WIFI_SSID is too long"),
        password: WIFI_PASSWORD.try_into().expect("WIFI_PASSWORD is too long"),
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    info!("[WIFI] Connected");
    Ok(wifi)
}
