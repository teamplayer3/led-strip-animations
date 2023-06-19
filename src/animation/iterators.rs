use core::ops::Range;

use crate::indexing::LedId;

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

pub struct CurveBatchIterator<'a, I, S> {
    index: u16,
    animation_iter: ActiveRangeIter,
    actual_animation_part: Option<AnchoredRange>,
    animation_part_item_idx: Option<u16>,
    animation: AnimationType,
    animation_range: I,
    from_color: FromColoring,
    led_controller: &'a S,
    fade_cache: Option<Rc<RefCell<ColorCache>>>,
}

impl<'a, I, S> CurveBatchIterator<'a, I, S> {
    fn new(
        led_controller: &'a S,
        animation_iter: ActiveRangeIter,
        animation_range: I,
        animation: AnimationType,
        fade_cache: Option<Rc<RefCell<ColorCache>>>,
        from_color: FromColoring,
    ) -> Self {
        Self {
            index: 0,
            animation_iter,
            actual_animation_part: None,
            animation,
            animation_range,
            led_controller,
            animation_part_item_idx: None,
            fade_cache,
            from_color,
        }
    }
}

impl<'a, I, S> CurveBatchIterator<'a, I, S> {
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

impl<'a, I, S> Iterator for CurveBatchIterator<'a, I, S>
where
    I: Indexing,
{
    type Item = FadeIter<'a, <I as Indexing>::OutputIndex, S>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_state = self.update_current_iter_state();
        let (anchor, led_idx) = match current_state {
            Some(v) => v,
            None => return None,
        };
        let mut led_final_idx = self
            .animation_range
            .map(anchor + self.animation_part_item_idx.unwrap())
            .expect("Led index out of range");

        let ret = FadeIter {
            animation: self.animation,
            animation_led_index: led_idx,
            fade_cache: self.fade_cache.to_owned(),
            from_color: self.from_color,
            inner_iter: led_final_idx,
            led_controller: self.led_controller.to_owned(),
        };
        self.index += 1;

        Some(ret)
    }
}

pub struct FadeIter<'a, I, S> {
    inner_iter: I,
    animation_led_index: LedId,
    led_controller: &'a S,
    fade_cache: Option<Rc<RefCell<ColorCache>>>,
    from_color: FromColoring,
    animation: AnimationType,
}

impl<'a, I, S> FadeIter<'a, I, S>
where
    S: Strip,
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

        let from_color = match self.from_color {
            FromColoring::Dynamic => self
                .fade_cache
                .as_ref()
                .unwrap()
                .borrow()
                .load_color(general_idx)
                .unwrap(),
            FromColoring::Fixed(color) => color,
        };
        let new_color = match self.animation {
            AnimationType::FadeToColor(meta) => calculate_with_curve(
                &meta.curve,
                Ticks::from(meta.fade_len + 1),
                &from_color,
                &meta.to_color,
                Tick::from(self.animation.animation_len() - 1 - idx),
            ),
            AnimationType::Hilled(meta) => {
                let (front_fade, front_curve, back_fade, back_curve) = match meta.symmetry {
                    AnimationSymmetry::Symmetric(curve, fade_len) => {
                        (fade_len, curve, fade_len, curve)
                    }
                    AnimationSymmetry::Asymmetric(first_part, second_part) => (
                        first_part.fade_len,
                        first_part.curve,
                        second_part.fade_len,
                        second_part.curve,
                    ),
                };

                let end_section = idx <= back_fade + 1;
                let first_section = idx >= meta.animation_len() - (front_fade + 1);

                if first_section {
                    calculate_with_curve(
                        &back_curve,
                        Ticks::from(front_fade + 1),
                        &meta.peak_color,
                        &from_color,
                        Tick::from(idx - (self.animation.animation_len() - (front_fade + 1) - 1)),
                    )
                } else if end_section {
                    calculate_with_curve(
                        &front_curve,
                        Ticks::from(back_fade + 1),
                        &from_color,
                        &meta.peak_color,
                        Tick::from(idx),
                    )
                } else {
                    meta.peak_color
                }
            }
        };

        if let Some(cache) = self.fade_cache.as_ref() {
            if idx == 0 {
                let _ = cache.borrow_mut().remove_cache(general_idx);
            }
        }

        new_color
    }
}

impl<'a, I, S> Iterator for FadeIter<'a, I, S>
where
    I: ExactSizeIterator<Item = Index>,
    S: Strip,
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
                    let anchor =
                        self.general_animation_len - u16::try_from(animation_part.len()).unwrap();
                    let used_idx = u16::try_from(animation_part.len()).unwrap();

                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(used_idx, 0);
                    Some(ret)
                }
                _ => unimplemented!(),
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
                    let anchor =
                        self.general_animation_len - u16::try_from(animation_part.len()).unwrap();
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
                    let anchor =
                        self.general_animation_len - u16::try_from(animation_part.len()).unwrap();
                    let used_idx = u16::try_from(animation_part.len()).unwrap();
                    let ret = AnchoredRange::new(anchor, animation_part);
                    self.update(used_idx, 0);
                    Some(ret)
                }
                _ => unimplemented!(),
            }
        } else {
            let animation_part = self.animation_offset..self.active_animation_len;
            let ret = AnchoredRange::new(u16::try_from(self.anchor).unwrap(), animation_part);
            self.update(
                remaining_animation_len,
                0 + i32::from(remaining_animation_len),
            );
            Some(ret)
        }
    }
}
