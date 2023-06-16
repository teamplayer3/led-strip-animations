use core::{cell::RefCell, fmt::Debug, marker::PhantomData};

use alloc::{borrow::ToOwned, rc::Rc};

use crate::{
    color::{HSVColor, LedColoring},
    color_cache::ColorCache,
    curve::{calculate_with_curve, Curve},
    indexing::{Indexing, LedId},
    strip::Strip,
    timeline::{Tick, Ticks},
};

use super::{Animation, AnimationMeta};

type FadeCache = Rc<RefCell<ColorCache>>;

#[derive(Debug)]
pub struct StaticAnimation<I, L> {
    duration: Ticks,
    range: I,
    to: HSVColor,
    curve: Curve,
    fade_cache: FadeCache,
    _led_controller: PhantomData<L>,
}

impl<I, L> StaticAnimation<I, L> {
    /// duration != 0, min. 1
    pub fn new(duration: Ticks, range: I, to: HSVColor, curve: Curve) -> Self {
        Self {
            duration,
            range,
            to,
            curve,
            fade_cache: Rc::new(RefCell::new(ColorCache::new())),
            _led_controller: Default::default(),
        }
    }
}

impl<I, S> Animation for StaticAnimation<I, S>
where
    I: Indexing + Clone + 'static,
    S: Strip,
{
    type Strip = S;

    type Iter = SingleBatchIterator<I>;

    fn animate(
        &self,
        current_tick: Tick,
        led_controller: &Self::Strip,
        _: &AnimationMeta,
    ) -> Self::Iter {
        if self.fade_cache.borrow().cache_size() == 0 {
            for i in 0..self.range.len() {
                let led_idx = self
                    .range
                    .index(LedId::try_from(i).unwrap())
                    .unwrap()
                    .next()
                    .unwrap();
                let _ = self
                    .fade_cache
                    .borrow_mut()
                    .cache_color(led_idx, &led_controller.get_color_of_led(led_idx).into());
            }
        }
        SingleBatchIterator::from_batch(
            self.range.to_owned(),
            self.to,
            self.duration,
            self.curve.to_owned(),
            self.fade_cache.to_owned(),
            current_tick,
        )
    }

    fn duration(&self) -> Ticks {
        self.duration
    }
}

pub struct SingleBatchIterator<I> {
    range: I,
    to_color: HSVColor,
    duration: Ticks,
    index: LedId,
    fade_cache: FadeCache,
    curve: Curve,
    current_tick: Tick,
}

impl<I> SingleBatchIterator<I> {
    fn from_batch(
        range: I,
        to_color: HSVColor,
        duration: Ticks,
        curve: Curve,
        fade_cache: FadeCache,
        current_tick: Tick,
    ) -> Self {
        Self {
            range,
            duration,
            to_color,
            index: 0,
            fade_cache,
            curve,
            current_tick,
        }
    }
}

impl<I> Iterator for SingleBatchIterator<I>
where
    I: Indexing,
{
    type Item = LedColoring<HSVColor>;

    fn next(&mut self) -> Option<Self::Item> {
        if usize::from(self.index) >= self.range.len() {
            return None;
        }

        let led_idx = self.range.index(self.index).unwrap().next().unwrap();
        let from_color = self.fade_cache.borrow().load_color(led_idx).unwrap();

        let next_color = calculate_with_curve(
            &self.curve,
            self.duration,
            &from_color,
            &self.to_color,
            self.current_tick,
        );

        let ret = Some(LedColoring::new(led_idx, next_color));

        self.index += 1;
        ret
    }
}

#[cfg(test)]
mod test {

    use crate::{
        animation::testing::{AnimationTester, Iterations},
        color::HSVColor,
        curve::Curve,
        mock::SPI,
        strip::mock::LedStrip,
    };

    use super::StaticAnimation;

    #[test]
    fn switch_leds_on() {
        let color = HSVColor::new(100, 0, 100);
        let mut led_controller = LedStrip::<SPI, 6>::new();
        let animation = StaticAnimation::new(1, 0..6, color, Curve::Step);

        let mut animation_tester =
            AnimationTester::new(animation, Iterations::Single, &mut led_controller);
        animation_tester.assert_state(1, (0..6).map(|led| (led, HSVColor::new(100, 0, 100))));
    }
}