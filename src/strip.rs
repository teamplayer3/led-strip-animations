use crate::{color::Color, indexing::LedId};

pub trait Strip {
    const LED_AMOUNT: usize;
    fn set_led_to_color(&mut self, led_id: LedId, color: &Color);
    fn set_leds_to_color(&mut self, led_ids: &[LedId], color: &Color);
    fn update_leds(&mut self);
    fn get_color_of_led(&self, led_id: LedId) -> Color;
}

#[cfg(test)]
pub(crate) mod mock {
    use core::{fmt::Debug, marker::PhantomData};

    use crate::{color::Color, indexing::LedId};

    use super::Strip;

    pub struct LedStrip<SPI, const N: usize> {
        leds: [Color; N],
        _spi: PhantomData<SPI>,
    }

    impl<SPI, const N: usize> LedStrip<SPI, N> {
        pub fn new() -> Self {
            Self {
                leds: [Color::init(0, 0, 0); N],
                _spi: Default::default(),
            }
        }
    }

    impl<SPI, const N: usize> Strip for LedStrip<SPI, N> {
        const LED_AMOUNT: usize = N;

        fn set_led_to_color(&mut self, led_id: LedId, color: &Color) {
            self.leds[usize::from(led_id)] = *color;
        }

        fn set_leds_to_color(&mut self, led_ids: &[LedId], color: &Color) {
            led_ids
                .iter()
                .for_each(|led_id| self.set_led_to_color(*led_id, color))
        }

        fn update_leds(&mut self) {}

        fn get_color_of_led(&self, led_id: LedId) -> Color {
            self.leds[usize::from(led_id)]
        }
    }

    impl<SPI, const N: usize> Debug for LedStrip<SPI, N> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("LedController").finish()
        }
    }
}
