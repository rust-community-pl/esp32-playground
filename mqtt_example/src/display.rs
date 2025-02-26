use crate::config::{CHAR_STYLE, CHAR_STYLE_SELECTED, DISPLAY_OFFSET, DISPLAY_SIZE, TEXTBOX_STYLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable};
use embedded_graphics::text::Text;
use embedded_hal::spi::MODE_3;
use embedded_text::TextBox;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::hal::units::*;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use mipidsi::options::ColorInversion;
use mipidsi::{Builder, Display};

type DisplaySpiInterface<'spi, DC> =
    SpiInterface<'spi, SpiDeviceDriver<'spi, SpiDriver<'spi>>, PinDriver<'spi, DC, Output>>;

type SpiDisplay<'display, DC, MODEL, RST> =
    Display<DisplaySpiInterface<'display, DC>, MODEL, PinDriver<'display, RST, Output>>;

pub struct QuizDisplay<'display, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    display: SpiDisplay<'display, DC, ST7789, RST>,
    backlight: PinDriver<'display, BL, Output>,
}
impl<'display, DC, RST, BL> QuizDisplay<'display, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spi: impl Peripheral<P = impl SpiAnyPins> + 'display,
        sclk: impl Peripheral<P = impl OutputPin> + 'display,
        sdo: impl Peripheral<P = impl OutputPin> + 'display,
        cs: impl Peripheral<P = impl OutputPin> + 'display,
        dc: impl Peripheral<P = DC> + 'display,
        rst: impl Peripheral<P = RST> + 'display,
        bl: impl Peripheral<P = BL> + 'display,
        buffer: &'display mut [u8],
    ) -> Self {
        let backlight: PinDriver<'display, BL, Output> = PinDriver::output(bl).unwrap();

        let spi_interface = Self::configure_spi(spi, sclk, sdo, cs, dc, buffer);

        let mut delay = Ets;
        let display = Builder::new(ST7789, spi_interface)
            .invert_colors(ColorInversion::Inverted)
            .reset_pin(PinDriver::output(rst).unwrap())
            .display_offset(DISPLAY_OFFSET.0, DISPLAY_OFFSET.1)
            .display_size(DISPLAY_SIZE.0, DISPLAY_SIZE.1)
            .init(&mut delay)
            .expect("Failed to init display");

        Self { display, backlight }
    }

    fn configure_spi(
        spi: impl Peripheral<P = impl SpiAnyPins> + 'display,
        sclk: impl Peripheral<P = impl OutputPin> + 'display,
        sdo: impl Peripheral<P = impl OutputPin> + 'display,
        cs: impl Peripheral<P = impl OutputPin> + 'display,
        dc: impl Peripheral<P = DC> + 'display,
        buffer: &'display mut [u8],
    ) -> DisplaySpiInterface<'display, DC> {
        let config = SpiConfig::new().baudrate(26.MHz().into()).data_mode(MODE_3);
        let spi_device = SpiDeviceDriver::new_single(
            spi,
            sclk,
            sdo,
            Option::<AnyIOPin>::None,
            Some(cs),
            &SpiDriverConfig::new(),
            &config,
        )
        .unwrap();

        SpiInterface::new(spi_device, PinDriver::output(dc).unwrap(), buffer)
    }
}

pub trait DisplayControls {
    fn clear(&mut self);
    fn on(&mut self);
    fn off(&mut self);
}

impl<DC, RST, BL> DisplayControls for QuizDisplay<'_, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    fn clear(&mut self) {
        self.display.clear(Rgb565::BLACK).ok();
    }

    fn on(&mut self) {
        self.backlight.set_high().ok();
    }

    fn off(&mut self) {
        self.backlight.set_low().ok();
    }
}

pub trait QuizRenderer {
    fn draw_question(&mut self, question: &str);
    fn draw_options(&mut self, options: &[String], selected: u8);
    fn draw_answer_sent(&mut self);
}

impl<DC, RST, BL> QuizRenderer for QuizDisplay<'_, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    fn draw_question(&mut self, question: &str) {
        TextBox::with_textbox_style(
            question,
            Rectangle::new(
                Point::zero(),
                Size::new(DISPLAY_SIZE.0.into(), (DISPLAY_SIZE.1 / 2).into()),
            ),
            CHAR_STYLE,
            TEXTBOX_STYLE,
        )
        .draw(&mut self.display)
        .ok();
    }

    fn draw_options(&mut self, options: &[String], selection: u8) {
        // Draw a black rectangle where the selection arrow is,
        // so we don't have to re-render the whole screen.
        Rectangle::new(
            Point::new(0, (DISPLAY_SIZE.1 / 2 - 12).into()),
            Size::new(10, 80),
        )
        .draw_styled(&PrimitiveStyle::with_fill(Rgb565::BLACK), &mut self.display)
        .ok();

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
            .draw(&mut self.display)
            .ok();
        }
    }

    fn draw_answer_sent(&mut self) {
        TextBox::with_textbox_style(
            "Answer sent!",
            Rectangle::new(
                Point::zero(),
                Size::new(DISPLAY_SIZE.0.into(), (DISPLAY_SIZE.1 / 2).into()),
            ),
            CHAR_STYLE,
            TEXTBOX_STYLE,
        )
        .draw(&mut self.display)
        .ok();
    }
}
