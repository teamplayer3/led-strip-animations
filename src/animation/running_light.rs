use core::{cell::RefCell, ops::Range};

use alloc::{boxed::Box, rc::Rc};

use crate::{
    color::{blend_colors, BlendMode, HSVColor, LedColoring},
    color_cache::ColorCache,
    indexing::{Index, Indexing, LedId},
    pattern::{Pattern, Progress},
    strip::Strip,
    timeline::{Tick, Ticks},
};

use super::{Animation, AnimationMeta};

#[derive(Debug, Clone, Copy)]
pub enum AnimationLen {
    FullStretch,
    Static(u16),
}

#[derive(Debug, Clone, Copy)]
enum BorderType {
    ClosedStartEnd,
    // WrappingStart,
    // WrappingEnd,
    WrappingStartEnd,
}

#[derive(Debug)]
pub struct RunningLight<I, P> {
    duration: Ticks,
    range: I,
    start_offset: i16,
    pattern: P,
    len: AnimationLen,
    border_wrapping: BorderType,
    fade_cache: Option<Rc<RefCell<ColorCache>>>,
    blend_mode: BlendMode,
}

impl<I, P: Pattern> RunningLight<I, P> {
    pub fn new(
        duration: Ticks,
        range: I,
        pattern: P,
        len: AnimationLen,
        start_offset: i16,
        circle: bool,
        blend_mode: BlendMode,
    ) -> Self {
        // TODO: init only if needed
        let fade_cache = Some(Rc::new(RefCell::new(ColorCache::new())));

        Self {
            fade_cache,
            duration,
            range,
            pattern,
            len,
            start_offset,
            border_wrapping: match circle {
                true => BorderType::WrappingStartEnd,
                false => BorderType::ClosedStartEnd,
            },
            blend_mode,
        }
    }
}

impl<S, I, P> Animation<S> for RunningLight<I, P>
where
    I: Indexing + Clone + 'static,
    S: Strip + 'static,
    P: Pattern<Color = HSVColor> + Clone + 'static,
{
    fn animate(
        &self,
        animation_tick: Tick,
        strip: Rc<RefCell<S>>,
        _: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>> {
        let animation_len = match self.len {
            AnimationLen::FullStretch => self.range.len() as u16,
            AnimationLen::Static(len) => len,
        };
        let jumps = calc_animation_jumps(&self.range, animation_len, self.border_wrapping);
        let act_jump = scale_time_to_jump(animation_tick, self.duration, jumps, self.start_offset);
        let start_led_id = scale_jump_to_animation_start(animation_len, act_jump);

        let animation_iter = ActiveRangeIter::new(
            start_led_id,
            animation_len,
            u16::try_from(self.range.len()).unwrap(),
            self.border_wrapping,
        );

        Box::new(
            CurveBatchIterator::new(
                strip,
                animation_iter,
                self.range.clone(),
                self.pattern.clone(),
                animation_len,
                self.fade_cache.clone(),
                self.blend_mode,
            )
            .flatten(),
        )
    }

    fn duration(&self) -> Ticks {
        self.duration
    }
}

fn calc_animation_jumps<I: Indexing>(range: &I, animation_len: u16, border: BorderType) -> u16 {
    let led_range_len = range.len() as u16;
    match border {
        BorderType::ClosedStartEnd => led_range_len + (animation_len - 2),
        BorderType::WrappingStartEnd => led_range_len - 1,
        // _ => unimplemented!(),
    }
}

fn scale_time_to_jump(time: Tick, duration: Ticks, jumps: u16, start_offset: i16) -> u16 {
    let jump = (time as f32 / (duration as f32 / jumps as f32)) as i16 + start_offset;
    if jump > jumps as i16 {
        jump.unsigned_abs() % jumps
    } else if jump < 0 {
        jumps - jump.unsigned_abs()
    } else {
        jump as u16
    }
}

fn scale_jump_to_animation_start(animation_len: u16, act_jump: u16) -> i32 {
    0 - (animation_len - 1) as i32 + act_jump as i32
}

type Anchor = u16;

#[derive(Debug, Clone)]
struct AnchoredRange {
    anchor: Anchor,
    range: Range<LedId>,
}

impl AnchoredRange {
    fn new(anchor: Anchor, range: Range<LedId>) -> Self {
        Self { anchor, range }
    }
}

pub struct CurveBatchIterator<I, S, P> {
    index: u16,
    animation_iter: ActiveRangeIter,
    actual_animation_part: Option<AnchoredRange>,
    animation_part_item_idx: Option<u16>,
    pattern: P,
    animation_len: u16,
    animation_range: I,
    led_controller: Rc<RefCell<S>>,
    fade_cache: Option<Rc<RefCell<ColorCache>>>,
    blend_mode: BlendMode,
}

impl<I, S, P> CurveBatchIterator<I, S, P> {
    fn new(
        led_controller: Rc<RefCell<S>>,
        animation_iter: ActiveRangeIter,
        animation_range: I,
        pattern: P,
        animation_len: u16,
        fade_cache: Option<Rc<RefCell<ColorCache>>>,
        blend_mode: BlendMode,
    ) -> Self {
        Self {
            index: 0,
            animation_iter,
            actual_animation_part: None,
            pattern,
            animation_len,
            animation_range,
            led_controller,
            animation_part_item_idx: None,
            fade_cache,
            blend_mode,
        }
    }
}

impl<I, S, P> CurveBatchIterator<I, S, P> {
    fn update_current_iter_state(&mut self) -> Option<(Anchor, LedId)> {
        loop {
            match self.actual_animation_part.take() {
                Some(mut a) => {
                    let anchor = a.anchor;
                    let led = a.range.next();
                    let _ =
                        self.animation_part_item_idx
                            .insert(match self.animation_part_item_idx {
                                Some(ref v) => v + 1,
                                _ => 0,
                            });
                    match led {
                        Some(idx) => {
                            let _ = self.actual_animation_part.insert(a);
                            break Some((anchor, idx));
                        }
                        None => continue,
                    }
                }
                None => {
                    let next = self.animation_iter.next();
                    self.animation_part_item_idx = None;
                    match next {
                        Some(a) => {
                            let _ = self.actual_animation_part.insert(a);
                        }
                        None => return None,
                    }
                }
            }
        }
    }
}

impl<I, S, P: Clone> Iterator for CurveBatchIterator<I, S, P>
where
    I: Indexing,
{
    type Item = FadeIter<<I as Indexing>::OutputIndex, S, P>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_state = self.update_current_iter_state();
        let (anchor, led_idx) = match current_state {
            Some(v) => v,
            None => return None,
        };
        let led_final_idx = self
            .animation_range
            .index(anchor + self.animation_part_item_idx.unwrap())
            .expect("Led index out of range");

        let ret = FadeIter {
            animation_led_index: led_idx,
            fade_cache: self.fade_cache.clone(),
            inner_iter: led_final_idx,
            led_controller: self.led_controller.clone(),
            pattern: self.pattern.clone(),
            animation_len: self.animation_len.clone(),
            blend_mode: self.blend_mode.clone(),
        };
        self.index += 1;

        Some(ret)
    }
}

pub struct FadeIter<I, S, P> {
    inner_iter: I,
    animation_led_index: LedId,
    led_controller: Rc<RefCell<S>>,
    fade_cache: Option<Rc<RefCell<ColorCache>>>,
    pattern: P,
    animation_len: u16,
    blend_mode: BlendMode,
}

impl<I, S, P> FadeIter<I, S, P>
where
    S: Strip,
    P: Pattern<Color = HSVColor>,
{
    fn cache_led_color(&self, fade_cache: &Rc<RefCell<ColorCache>>, general_idx: LedId) {
        let _ = fade_cache.borrow_mut().cache_color(
            general_idx,
            &self
                .led_controller
                .borrow()
                .get_color_of_led(general_idx)
                .into(),
        );
    }

    fn map_led_idx_to_color(&self, general_idx: LedId, idx: LedId) -> HSVColor {
        if let Some(cache) = self.fade_cache.as_ref() {
            self.cache_led_color(cache, general_idx);
        }

        let animation_color = self.pattern.color_at(Progress::new(
            self.animation_len - idx - 1,
            self.animation_len,
        ));

        let from_color = self
            .fade_cache
            .as_ref()
            .and_then(|c| Some(c.borrow().load_color(general_idx).unwrap()));

        let new_color = match from_color {
            Some(from) => blend_colors(from, animation_color, self.blend_mode),
            None => animation_color.color,
        };

        if let Some(cache) = self.fade_cache.as_ref() {
            if idx == 0 {
                let _ = cache.borrow_mut().remove_cache(general_idx);
            }
        }

        new_color
    }
}

impl<I, S, P> Iterator for FadeIter<I, S, P>
where
    I: ExactSizeIterator<Item = Index>,
    S: Strip,
    P: Pattern<Color = HSVColor>,
{
    type Item = LedColoring<HSVColor>;

    fn next(&mut self) -> Option<Self::Item> {
        let led_id = self.inner_iter.next();

        if let Some(led_id) = led_id {
            let color = self.map_led_idx_to_color(led_id, self.animation_led_index);
            return Some(LedColoring::new(led_id, color));
        }

        None
    }
}

struct ActiveRangeIter {
    // is changing while iterating
    anchor: i32,
    active_animation_len: u16,
    general_animation_len: u16,
    border_type: BorderType,
    // is changing while iterating
    animation_offset: u16,
}

impl ActiveRangeIter {
    fn new(
        anchor: i32,
        active_animation_len: u16,
        general_animation_len: u16,
        border_type: BorderType,
    ) -> Self {
        Self {
            anchor,
            active_animation_len,
            general_animation_len,
            border_type,
            animation_offset: 0,
        }
    }

    fn update(&mut self, used_len_in_iter: u16, move_anchor_to: i32) {
        self.animation_offset += used_len_in_iter;
        self.anchor = move_anchor_to;
    }
}

impl Iterator for ActiveRangeIter {
    type Item = AnchoredRange;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining_animation_len = self.active_animation_len - self.animation_offset;
        if remaining_animation_len == 0 {
            return None;
        }

        let an_start_outside = self.anchor < 0;
        let an_end_outside = self.anchor + i32::from(remaining_animation_len)
            > i32::from(self.general_animation_len);

        if an_start_outside && an_end_outside {
            unimplemented!()
        } else if an_start_outside {
            let outside_len = u16::try_from(self.anchor.abs()).unwrap();
            match self.border_type {
                BorderType::ClosedStartEnd => {
                    // cut all what is before 0, this will be the last returned item,
                    // set anchor to start (0) and return a range containing indices
                    // [outside_len + animation_offset..end].
                    // example: anchor = -3, animation_len = 5
                    //          returns: {0, 1, 2} | [3, 4] with anchor 0
                    let animation_part =
                        (outside_len + self.animation_offset)..self.active_animation_len;
                    let anchor = 0;
                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(remaining_animation_len, 0);
                    Some(ret)
                }
                BorderType::WrappingStartEnd => {
                    // everything what is bevor 0 will be wrapped at the "end" of the animation
                    // range. anchor will be set to 0, a range [animation_offset..outside_len]
                    // example: anchor= -2, general_an_len = 6, animation_len = 5
                    //          returns: [0, 1] | {2, 3, 4} with anchor 4
                    let animation_part = self.animation_offset..outside_len;
                    let anchor = self.general_animation_len
                        - u16::try_from(ExactSizeIterator::len(&animation_part)).unwrap();
                    let used_idx = u16::try_from(ExactSizeIterator::len(&animation_part)).unwrap();

                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(used_idx, 0);
                    Some(ret)
                }
            }
        } else if an_end_outside {
            let outside_len = (u16::try_from(self.anchor).unwrap() + remaining_animation_len)
                - self.general_animation_len;
            match self.border_type {
                BorderType::ClosedStartEnd => {
                    // end will be cut, this will be the last returned item, anchor is set to
                    // general_an_len + 1, returns a range [animation_offset..animation_len - outside_len]
                    // example: anchor 4, general_an_len = 6, animation_len = 5
                    //          returns: [0, 1] | {2, 3, 4} with anchor 4
                    let animation_part =
                        self.animation_offset..(self.active_animation_len - outside_len);
                    let anchor = self.general_animation_len
                        - u16::try_from(ExactSizeIterator::len(&animation_part)).unwrap();
                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(
                        remaining_animation_len,
                        i32::from(self.general_animation_len),
                    );
                    Some(ret)
                }
                BorderType::WrappingStartEnd => {
                    // end will be wrapped to start, anchor is set to 0, returns a range
                    // [animation_offset..animation_len - outside_len]
                    // example: anchor 4, general_an_len = 6, animation_len = 5
                    //          returns: [0, 1] | {2, 3, 4} with anchor 4
                    let animation_part =
                        self.animation_offset..(self.active_animation_len - outside_len);
                    let anchor = self.general_animation_len
                        - u16::try_from(ExactSizeIterator::len(&animation_part)).unwrap();
                    let used_idx = u16::try_from(ExactSizeIterator::len(&animation_part)).unwrap();
                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(used_idx, 0);
                    Some(ret)
                }
            }
        } else {
            let animation_part = self.animation_offset..self.active_animation_len;
            let ret = AnchoredRange::new(u16::try_from(self.anchor).unwrap(), animation_part);
            self.update(remaining_animation_len, i32::from(remaining_animation_len));
            Some(ret)
        }
    }
}

#[cfg(test)]
mod test {

    use core::cell::RefCell;

    use alloc::rc::Rc;
    use assert_matches::assert_matches;

    use crate::{
        animation::testing::{AnimationTester, Iterations},
        color::{HSVColor, TransparentColor},
        curve::{calculate_with_curve, Curve},
        mock::SPI,
        pattern::HillPattern,
        strip::mock::LedStrip,
    };

    use super::*;

    #[test]
    fn test_indexed_range_iter_closed_start_end() {
        let border_type = BorderType::ClosedStartEnd;
        let general_animation_len = 7;
        let animation_len = 5;

        // start before 0
        let anchor = -2;
        let mut iter =
            ActiveRangeIter::new(anchor, animation_len, general_animation_len, border_type);
        assert_matches!(iter.next(), Some(i) if i.anchor == 0 && i.range == (2u16..5));
        assert_matches!(iter.next(), None);

        // all in animation range
        let anchor = 1;
        let mut iter =
            ActiveRangeIter::new(anchor, animation_len, general_animation_len, border_type);
        assert_matches!(iter.next(), Some(i) if i.anchor == 1 && i.range == (0u16..5));
        assert_matches!(iter.next(), None);

        // ends after animation len
        let anchor = 5;
        let mut iter =
            ActiveRangeIter::new(anchor, animation_len, general_animation_len, border_type);
        assert_matches!(iter.next(), Some(i) if i.anchor == 5 && i.range == (0u16..2));
        assert_matches!(iter.next(), None)
    }

    #[test]
    fn test_indexed_range_iter_wrapping_start_end() {
        let border_type = BorderType::WrappingStartEnd;
        let general_animation_len = 7;

        // start
        let anchor = -2;
        let animation_len = 4;
        let mut iter =
            ActiveRangeIter::new(anchor, animation_len, general_animation_len, border_type);
        assert_matches!(iter.next(), Some(i) if i.anchor == 5 && i.range == (0u16..2));
        assert_matches!(iter.next(), Some(i) if i.anchor == 0 && i.range == (2u16..4));
        assert_matches!(iter.next(), None);

        // end
        let anchor = 5;
        let animation_len = 4;
        let mut iter =
            ActiveRangeIter::new(anchor, animation_len, general_animation_len, border_type);
        assert_matches!(iter.next(), Some(i) if i.anchor == 5 && i.range == (0u16..2));
        assert_matches!(iter.next(), Some(i) if i.anchor == 0 && i.range == (2u16..4));
        assert_matches!(iter.next(), None);
    }

    #[test]
    fn test_color_fade_curve() {
        let len = 2;
        let curve = Curve::Linear;
        let from = HSVColor::new(0, 0, 0);
        let to = HSVColor::new(100, 0, 0);

        assert_eq!(
            calculate_with_curve(&curve, len, &from, &to, 0),
            HSVColor::new(0, 0, 0)
        );
        assert_eq!(
            calculate_with_curve(&curve, len, &from, &to, 1),
            HSVColor::new(50, 0, 0)
        );
        assert_eq!(
            calculate_with_curve(&curve, len, &from, &to, 2),
            HSVColor::new(100, 0, 0)
        );
    }

    // #[test]
    // fn test_animate_running_light_fade_to() {
    //     let led_controller = Rc::new(RefCell::new(LedStrip::<SPI, 20>::new()));
    //     let range = 6u16..10;
    //     let duration = 40;
    //     let animation = AnimationType::FadeToColor(FadeToAnimationMeta {
    //         curve: Curve::Linear,
    //         fade_len: 1,
    //         to_color: HSVColor::new(100, 0, 0),
    //     });

    //     let animation = RunningLight::new(
    //         duration,
    //         range,
    //         FromColoring::Fixed(HSVColor::new(0, 0, 0)),
    //         0,
    //         false,
    //         animation,
    //     );
    //     let mut animation_tester =
    //         AnimationTester::new(animation, Iterations::Single, led_controller);

    //     animation_tester.assert_state(0, [(6, HSVColor::new(0, 0, 0))]);

    //     animation_tester.assert_state(
    //         8,
    //         [(6, HSVColor::new(50, 0, 0)), (7, HSVColor::new(0, 0, 0))],
    //     );

    //     animation_tester.assert_state(
    //         16,
    //         [
    //             (6, HSVColor::new(100, 0, 0)),
    //             (7, HSVColor::new(50, 0, 0)),
    //             (8, HSVColor::new(0, 0, 0)),
    //         ],
    //     );

    //     animation_tester.assert_state(
    //         32,
    //         [(8, HSVColor::new(100, 0, 0)), (9, HSVColor::new(50, 0, 0))],
    //     );

    //     animation_tester.assert_state(40, [(9, HSVColor::new(100, 0, 0))]);
    // }

    // #[test]
    // fn test_animate_running_light_fade_to_wrapped() {
    //     let led_controller = Rc::new(RefCell::new(LedStrip::<SPI, 20>::new()));
    //     let range = 6u16..10;
    //     let duration = 40;
    //     let animation = AnimationType::FadeToColor(FadeToAnimationMeta {
    //         curve: Curve::Linear,
    //         fade_len: 1,
    //         to_color: HSVColor::new(100, 0, 0),
    //     });

    //     let animation = RunningLight::new(
    //         duration,
    //         range,
    //         FromColoring::Fixed(HSVColor::new(0, 0, 0)),
    //         0,
    //         true,
    //         animation,
    //     );
    //     let mut animation_tester =
    //         AnimationTester::new(animation, Iterations::Single, led_controller);

    //     animation_tester.assert_state(
    //         0,
    //         [
    //             (8, HSVColor::new(100, 0, 0)),
    //             (9, HSVColor::new(50, 0, 0)),
    //             (6, HSVColor::new(0, 0, 0)),
    //         ],
    //     );

    //     animation_tester.assert_state(
    //         14,
    //         [
    //             (9, HSVColor::new(100, 0, 0)),
    //             (6, HSVColor::new(50, 0, 0)),
    //             (7, HSVColor::new(0, 0, 0)),
    //         ],
    //     );

    //     animation_tester.assert_state(
    //         40,
    //         [
    //             (7, HSVColor::new(100, 0, 0)),
    //             (8, HSVColor::new(50, 0, 0)),
    //             (9, HSVColor::new(0, 0, 0)),
    //         ],
    //     );
    // }

    #[test]
    fn test_animate_running_light_hilled() {
        let led_controller = Rc::new(RefCell::new(LedStrip::<SPI, 20>::new()));
        let range = 6u16..10;
        let duration = 40;

        let pattern = HillPattern::new(
            1,
            TransparentColor::opaque(HSVColor::new(100, 100, 100)),
            Curve::Linear,
        );

        let animation = RunningLight::new(
            duration,
            range,
            pattern,
            AnimationLen::Static(5),
            0,
            false,
            BlendMode::AllChannels,
        );
        let mut animation_tester =
            AnimationTester::new(animation, Iterations::Single, led_controller);

        animation_tester.assert_state(0, [(6, HSVColor::new(0, 0, 0))]);

        animation_tester.assert_state(
            8,
            [(6, HSVColor::new(50, 50, 50)), (7, HSVColor::new(0, 0, 0))],
        );

        animation_tester.assert_state(
            16,
            [
                (6, HSVColor::new(100, 100, 100)),
                (7, HSVColor::new(50, 50, 50)),
                (8, HSVColor::new(0, 0, 0)),
            ],
        );

        animation_tester.assert_state(
            32,
            [
                (7, HSVColor::new(0, 0, 0)),
                (8, HSVColor::new(50, 50, 50)),
                (9, HSVColor::new(100, 100, 100)),
            ],
        );

        animation_tester.assert_state(40, [(9, HSVColor::new(0, 0, 0))]);
    }

    // #[test]
    // fn test_animate_running_light_hilled_circled() {
    //     let led_controller = Rc::new(RefCell::new(LedController::<SPI, 20>::new()));
    //     let range = 6u16..10;
    //     let duration = 40;
    //     let animation = AnimationType::Hilled(HilledAnimationMeta {
    //         peak_color: HSVColor::new(100, 0, 100),
    //         peak_len: 2,
    //         symmetry: AnimationSymmetry::Symmetric(Curve::Linear, 1),
    //     });

    //     let animation = RunningLight::new(
    //         duration,
    //         range,
    //         FromColoring::Fixed(HSVColor::new(0, 0, 0)),
    //         0,
    //         true,
    //         animation,
    //     );
    //     let mut animation_tester =
    //         AnimationTester::new(animation, Iterations::Single, led_controller);
    //     animation_tester.assert_state(0, [(6, HSVColor::new(100, 0, 0))]);

    //     animation_tester.assert_state(
    //         8,
    //         [
    //             (6, HSVColor::new(100, 0, 50)),
    //             (7, HSVColor::new(100, 0, 0)),
    //         ],
    //     );

    //     animation_tester.assert_state(
    //         16,
    //         [
    //             (6, HSVColor::new(100, 0, 100)),
    //             (7, HSVColor::new(100, 0, 100)),
    //             (8, HSVColor::new(100, 0, 50)),
    //             (9, HSVColor::new(100, 0, 0)),
    //         ],
    //     );

    //     animation_tester.assert_state(
    //         32,
    //         [
    //             (7, HSVColor::new(100, 0, 0)),
    //             (8, HSVColor::new(100, 0, 50)),
    //             (9, HSVColor::new(100, 0, 100)),
    //         ],
    //     );

    //     animation_tester.assert_state(40, [(9, HSVColor::new(100, 0, 0))]);
    // }
}
