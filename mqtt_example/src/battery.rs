use crate::event::DeviceEvent;
use esp_idf_svc::hal::adc::attenuation::DB_11;
use esp_idf_svc::hal::adc::oneshot::config::{AdcChannelConfig, Calibration};
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::Resolution;
use esp_idf_svc::hal::gpio::ADCPin;
use esp_idf_svc::hal::peripheral::Peripheral;
use log::info;
use std::sync::mpsc;
use std::thread;
use std::thread::{Scope, ScopedJoinHandle};
use std::time::Duration;

mod capacity_curve {
    use log::info;

    /// Naive const equivalent of `f.powi(n)`.
    /// Performance is not a problem, since this is evaluated only at compile time.
    /// Thanks to RFC 5314, basic floating point arithmetic is now stable in const
    const fn pow(f: f32, n: i32) -> f32 {
        let mut out: f32 = 1.0;
        let mut i = 0;
        while i < n {
            out *= f;
            i += 1;
        }
        out
    }

    /// Maps battery voltage (in mV) to battery capacity (in %).
    /// Calculated from battery datasheet using https://mycurvefit.com.
    const fn to_capacity(voltage: u16) -> u8 {
        const A: f32 = -0.22;
        // `B` is rounded from ~24.62 in order to use
        // deterministic version of `f32::powi`
        // (which is much easier to implement than `f32::powf`)
        const B: i32 = 25; // ~24.62;
        const C: f32 = 3662.97;
        const D: f32 = 104.42;

        let battery_level = (D + (A - D) / (1.0 + pow(voltage as f32 / C, B))) as i8;
        // `i8::clamp` as of March 2025 is not const
        if battery_level > 100 {
            return 100;
        } else if battery_level < 0 {
            return 0;
        }
        battery_level as u8
    }

    /// `LOOKUP_RESOLUTION`mV per step
    const LOOKUP_RESOLUTION: u16 = 10;
    const LOOKUP_SIZE: usize = 121;
    const LOOKUP_START: u16 = 3000;
    const LOOKUP_END: u16 = LOOKUP_START + (LOOKUP_SIZE as u16 - 1) * LOOKUP_RESOLUTION;
    /// Maps voltage to capacity %
    /// Where `0` corresponds to `LOOKUP_START`mV
    /// and `LOOKUP_SIZE-1` corresponds to `LOOKUP_END`mV
    const CAPACITY_LOOKUP: [u8; LOOKUP_SIZE] = {
        let mut lookup: [u8; LOOKUP_SIZE] = [0; LOOKUP_SIZE];
        // as of March 2025, for loops are not allowed in const context
        let mut i = 0;
        while i < LOOKUP_SIZE {
            lookup[i] = to_capacity(LOOKUP_START + i as u16 * LOOKUP_RESOLUTION);
            i += 1;
        }
        lookup
    };

    /// Battery is connected to ADC pin through a voltage divider:
    /// `V_ADC = R7/(R6+R7) * V_BAT`
    /// `ADC_MULTIPLIER = (R6+R7)/R7`
    const ADC_MULTIPLIER: u8 = 2;
    #[inline]
    fn get_battery_voltage(adc_reading: u16) -> u16 {
        adc_reading * ADC_MULTIPLIER as u16
    }

    /// If battery is charging, the ADC reading will show very high readings, above 4.2V.
    /// This would cause the battery level to always display 100%, when charging.
    /// There is no way to change this behaviour without hardware modifications.
    /// We can still naively check for unusually high voltage and assume that battery is charging.
    const CHARGING_THRESHOLD: u16 = 4300;

    /// Maps `adc_reading` to battery capacity (in %).
    /// Returns `None`, if battery is charging.
    #[inline]
    pub fn get_battery_level(adc_reading: u16) -> Option<u8> {
        let voltage = get_battery_voltage(adc_reading);
        info!("[Battery reader] Voltage: {:.2}V", voltage as f32 / 1000.0);
        if voltage > CHARGING_THRESHOLD {
            return None;
        }
        let lookup_index =
            ((voltage.clamp(LOOKUP_START, LOOKUP_END) - LOOKUP_START) / LOOKUP_RESOLUTION) as usize;
        Some(CAPACITY_LOOKUP[lookup_index])
    }
}

pub fn spawn_reader_thread<'scope, T>(
    scope: &'scope Scope<'scope, '_>,
    adc: impl Peripheral<P = T::Adc> + 'scope + Send,
    battery_pin: impl Peripheral<P = T> + 'scope + Send,
    sender: mpsc::Sender<DeviceEvent>,
) -> Result<ScopedJoinHandle<'scope, ()>, std::io::Error>
where
    T: ADCPin,
{
    thread::Builder::new()
        .stack_size(8192)
        .spawn_scoped(scope, move || {
            info!("[Battery reader] Starting...");
            let adc_driver = AdcDriver::new(adc).unwrap();
            let mut bat_adc_channel = AdcChannelDriver::new(
                &adc_driver,
                battery_pin,
                &AdcChannelConfig {
                    attenuation: DB_11,
                    calibration: Calibration::Line,
                    resolution: Resolution::Resolution12Bit,
                },
            )
            .unwrap();
            loop {
                if let Ok(reading) = adc_driver.read(&mut bat_adc_channel) {
                    let battery_level = capacity_curve::get_battery_level(reading);
                    sender
                        .send(DeviceEvent::BatteryLevel {
                            data: battery_level,
                        })
                        .ok();
                    match battery_level {
                        Some(_) => {
                            thread::sleep(Duration::from_secs(60));
                        }
                        None => {
                            thread::sleep(Duration::from_secs(2));
                        }
                    }
                }
            }
        })
}
