use core::{cell::RefCell, fmt::Debug};

use alloc::{boxed::Box, rc::Rc};

use crate::{
    color::{blend_colors, BlendMode, HSVColor, LedColoring, Spectrum, TransparentColor},
    color_cache::ColorCache,
    curve::{calculate_with_curve, Curve},
    indexing::{Index, Indexing, LedId},
    strip::Strip,
    timeline::{Tick, Ticks},
};

use super::{Animation, AnimationMeta};

type FadeCache = Rc<RefCell<ColorCache>>;

#[derive(Debug)]
pub struct StaticAnimation<I, SP> {
    duration: Ticks,
    range: I,
    to: SP,
    curve: Curve,
    fade_cache: FadeCache,
    blend_mode: BlendMode,
}

impl<I, SP> StaticAnimation<I, SP>
where
    SP: Spectrum,
{
    /// duration != 0, min. 1
    pub fn new(duration: Ticks, range: I, to: SP, curve: Curve, blend_mode: BlendMode) -> Self {
        Self {
            duration,
            range,
            to,
            curve,
            fade_cache: Rc::new(RefCell::new(ColorCache::new())),
            blend_mode,
        }
    }
}

impl<I, SP> StaticAnimation<I, SP> {
    fn cache_current_colors<S>(&self, led_controller: Rc<RefCell<S>>)
    where
        I: Indexing,
        S: Strip,
    {
        for i in 0..self.range.len() {
            let mut output_index = self.range.index(LedId::try_from(i).unwrap()).unwrap();

            for _ in 0..output_index.len() {
                let led_idx = output_index.next().unwrap();
                let _ = self.fade_cache.borrow_mut().cache_color(
                    led_idx,
                    &led_controller.borrow().get_color_of_led(led_idx).into(),
                );
            }
        }
    }
}

impl<S, I, SP> Animation<S> for StaticAnimation<I, SP>
where
    I: Indexing + Clone + 'static,
    S: Strip,
    SP: Spectrum<Color = HSVColor> + Clone + 'static,
{
    fn animate(
        &self,
        animation_tick: Tick,
        led_controller: Rc<RefCell<S>>,
        _: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>> {
        if self.fade_cache.borrow().cache_size() == 0 {
            self.cache_current_colors(led_controller.clone());
        }

        Box::new(
            SingleBatchIterator::from_batch(
                self.range.clone(),
                self.to.clone(),
                self.duration,
                self.curve.clone(),
                self.fade_cache.clone(),
                animation_tick,
                self.blend_mode,
            )
            .flatten(),
        )
    }

    fn duration(&self) -> Ticks {
        self.duration
    }
}

pub struct SingleBatchIterator<I, SP> {
    range: I,
    to_color: SP,
    duration: Ticks,
    index: LedId,
    fade_cache: FadeCache,
    curve: Curve,
    current_tick: Tick,
    blend_mode: BlendMode,
}

impl<I, SP> SingleBatchIterator<I, SP> {
    fn from_batch(
        range: I,
        to_color: SP,
        duration: Ticks,
        curve: Curve,
        fade_cache: FadeCache,
        current_tick: Tick,
        blend_mode: BlendMode,
    ) -> Self {
        Self {
            range,
            duration,
            to_color,
            index: 0,
            fade_cache,
            curve,
            current_tick,
            blend_mode,
        }
    }
}

impl<I, SP> Iterator for SingleBatchIterator<I, SP>
where
    I: Indexing,
    SP: Spectrum<Color = HSVColor>,
{
    type Item = MapIterator<<I as Indexing>::OutputIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        if usize::from(self.index) >= self.range.len() {
            return None;
        }

        let led_idx = self.range.index(self.index).unwrap();
        let percentage = self.index as f32 / self.range.len() as f32;
        let to_color = self.to_color.color_at(percentage);

        let ret = MapIterator {
            output_index: led_idx,
            curve: self.curve.clone(),
            duration: self.duration,
            current_tick: self.current_tick,
            fade_cache: self.fade_cache.clone(),
            to_color,
            blend_mode: self.blend_mode,
        };

        self.index += 1;
        Some(ret)
    }
}

pub struct MapIterator<O> {
    output_index: O,
    curve: Curve,
    duration: Ticks,
    current_tick: Tick,
    fade_cache: FadeCache,
    to_color: TransparentColor<HSVColor>,
    blend_mode: BlendMode,
}

impl<O> Iterator for MapIterator<O>
where
    O: ExactSizeIterator<Item = Index>,
{
    type Item = LedColoring<HSVColor>;

    fn next(&mut self) -> Option<Self::Item> {
        let led = self.output_index.next();

        led.map(|led| {
            let from_color = self.fade_cache.borrow().load_color(led).unwrap();
            let to_color = blend_colors(from_color, self.to_color, self.blend_mode);

            let next_color = calculate_with_curve(
                &self.curve,
                self.duration,
                &from_color,
                &to_color,
                self.current_tick,
            );

            LedColoring::new(led, next_color)
        })
    }
}

#[cfg(test)]
mod test {

    use core::cell::RefCell;

    use alloc::rc::Rc;

    use crate::{
        animation::testing::{AnimationTester, Iterations},
        color::{BlendMode, HSVColor, TransparentColor},
        curve::Curve,
        mock::SPI,
        strip::{mock::LedStrip, Strip},
    };

    use super::StaticAnimation;

    #[test]
    fn switch_leds_on() {
        let color = HSVColor::new(100, 0, 100);
        let led_controller = Rc::new(RefCell::new(LedStrip::<SPI, 6>::new()));
        let animation = StaticAnimation::new(1, 0..6, color, Curve::Step, BlendMode::AllChannels);

        let mut animation_tester =
            AnimationTester::new(animation, Iterations::Single, led_controller);
        animation_tester.assert_state(1, (0..6).map(|led| (led, HSVColor::new(100, 0, 100))));
    }

    #[test]
    fn test_color_cache() {
        let led_controller = Rc::new(RefCell::new(LedStrip::<SPI, 6>::new()));
        led_controller
            .borrow_mut()
            .set_leds_to_color(&[0, 2, 4], &HSVColor::new(0, 100, 100).into());
        led_controller
            .borrow_mut()
            .set_leds_to_color(&[1, 3, 5], &HSVColor::new(60, 100, 100).into());

        let animation = StaticAnimation::new(
            1,
            0..6,
            TransparentColor::new(HSVColor::new(100, 100, 100), 0.5),
            Curve::Step,
            BlendMode::AllChannels,
        );

        let mut animation_tester =
            AnimationTester::new(animation, Iterations::Single, led_controller);
        animation_tester.assert_state(
            1,
            [0, 2, 4]
                .into_iter()
                .map(|led| {
                    [
                        (led, HSVColor::new(50, 100, 100)),
                        (led + 1, HSVColor::new(80, 100, 100)),
                    ]
                })
                .flatten(),
        );
    }
}
