use std::error::Error;
use std::thread;

use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};

use embedded_hal::spi::MODE_3;

use esp_idf_svc::hal::{
    delay::Ets,
    gpio::{AnyIOPin, PinDriver},
    peripherals::Peripherals,
    spi::{SpiConfig, SpiDeviceDriver, SpiDriverConfig},
    units::*,
};
use mipidsi::{interface::SpiInterface, models::ST7789, Builder};
use tinybmp::Bmp;

use mipidsi::options::ColorInversion;

// It is important to define display offsets and size so that
// the image and the text renders correctly on the display
const DISPLAY_OFFSET: (u16, u16) = (52, 40);
const DISPLAY_SIZE: (u16, u16) = (135, 240);

fn main() -> Result<(), Box<dyn Error>> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    // Turn on display backlight
    // GPIO configuration here works simillarly as in C++ coding paradigm
    // Select the role of the pin (here output)
    // and then set the pin to the wanted state (here high)
    let mut backlight = PinDriver::output(peripherals.pins.gpio4)?;
    backlight.set_high()?;

    // Define SPI configuration
    // Before the initialization the display,
    // first define the SPI pin configuration
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
    // Here, the display is finally initialized
    // The device used is ST7789
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

    // Draw image
    let bmp_data = include_bytes!("rustmeet.bmp");
    let bmp = Bmp::from_slice(bmp_data).expect("Failed to parse bitmap");
    Image::new(&bmp, Point::new(0, 52))
        .draw(&mut display)
        .expect("Failed to draw image");

    // Draw text
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Text::new("rustmeet.eu", Point::new(34, 230), style)
        .draw(&mut display)
        .expect("Failed to draw text");

    // Do nothing, forever
    thread::park();
    Ok(())
}
