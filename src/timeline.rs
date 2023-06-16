use crate::{
    animation::{Animation, BoxedAnimation},
    color::{HSVColor, LedColoring},
    strip::Strip,
};

use super::animation::TimedAnimation;

pub type Tick = u32;
pub type Ticks = u32;

pub trait Timeline<S, A, I>
where
    A: Animation<Strip = S, Iter = I>,
    I: Iterator<Item = LedColoring<HSVColor>>,
    S: Strip + 'static,
{
    type Iter<'a>: Iterator<Item = &'a TimedAnimation<A>>
    where
        Self: 'a,
        A: 'a;

    fn get_current_entries<'a>(&'a self, current_tick: Tick) -> Self::Iter<'a>;
    fn has_finished(&self, current_tick: Tick) -> bool;
    fn should_repeat(&self) -> bool;
}

#[cfg(feature = "alloc")]
pub struct DynTimelineBuilder<S, I>
where
    S: Strip,
{
    animations: alloc::vec::Vec<TimedAnimation<BoxedAnimation<S, I>>>,
    repeating: bool,
}

#[cfg(feature = "alloc")]
impl<S, I> DynTimelineBuilder<S, I>
where
    S: Strip + 'static,
{
    pub fn new() -> Self {
        Self {
            animations: alloc::vec::Vec::new(),
            repeating: false,
        }
    }

    pub fn add_animation<A>(mut self, start: Tick, animation: A) -> Self
    where
        A: crate::animation::Animation<Strip = S, Iter = I> + 'static,
    {
        self.animations.push(TimedAnimation::new(
            start,
            alloc::boxed::Box::new(animation),
        ));
        self
    }

    pub fn repeating(mut self) -> Self {
        self.repeating = true;
        self
    }

    pub fn finish(self) -> DynTimeline<S, I> {
        let mut animations = self.animations;
        animations.sort_by(|a, b| {
            if a.0 < b.0 {
                core::cmp::Ordering::Less
            } else if a.0 > b.0 {
                core::cmp::Ordering::Greater
            } else {
                core::cmp::Ordering::Equal
            }
        });
        DynTimeline {
            entries: animations,
            repeating: self.repeating,
        }
    }
}

#[cfg(feature = "alloc")]
pub struct DynTimeline<S, I>
where
    S: Strip,
{
    entries: alloc::vec::Vec<TimedAnimation<crate::animation::BoxedAnimation<S, I>>>,
    repeating: bool,
}

#[cfg(feature = "alloc")]
impl<S, I> DynTimeline<S, I>
where
    S: Strip,
{
    pub fn new(repeating: bool) -> Self {
        let entries = { alloc::vec::Vec::new() };
        Self { entries, repeating }
    }
}

#[cfg(feature = "alloc")]
pub struct DynTimelineIter<'a, S, I>
where
    S: Strip,
{
    s: &'a [TimedAnimation<crate::animation::BoxedAnimation<S, I>>],
    act_index: usize,
    within_tick: Tick,
}

#[cfg(feature = "alloc")]
impl<'a, S, I> DynTimelineIter<'a, S, I>
where
    S: Strip,
{
    pub(crate) fn new(
        animations: &'a alloc::vec::Vec<TimedAnimation<crate::animation::BoxedAnimation<S, I>>>,
        within_tick: Tick,
    ) -> Self {
        Self {
            s: animations.as_slice(),
            act_index: 0,
            within_tick,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, S, I> Iterator for DynTimelineIter<'a, S, I>
where
    S: Strip,
    I: Iterator<Item = LedColoring<HSVColor>> + 'static,
{
    type Item = &'a TimedAnimation<BoxedAnimation<S, I>>;

    fn next(&mut self) -> Option<Self::Item> {
        let act_animation = loop {
            if self.s.len() <= self.act_index {
                break None;
            }
            let an = &self.s[self.act_index];
            if self.within_tick > an.0 + an.animation_duration() {
                self.act_index += 1;
            } else {
                break Some(an);
            }
        };
        act_animation.and_then(|act_animation| {
            self.act_index += 1;
            if act_animation.0 < self.within_tick {
                Some(act_animation)
            } else {
                None
            }
        })
    }
}

#[cfg(feature = "alloc")]
impl<S, I> Timeline<S, BoxedAnimation<S, I>, I> for DynTimeline<S, I>
where
    S: Strip + 'static,
    I: Iterator<Item = LedColoring<HSVColor>> + 'static,
{
    type Iter<'a> = DynTimelineIter<'a, S, I>;

    fn get_current_entries<'a>(&'a self, current_tick: Tick) -> Self::Iter<'a> {
        DynTimelineIter::new(&self.entries, current_tick)
    }

    fn has_finished(&self, act_tick: Tick) -> bool {
        let last_entry = self.entries.last();
        match last_entry {
            Some(e) => e.0 + e.1.duration() < act_tick,
            None => true,
        }
    }

    fn should_repeat(&self) -> bool {
        self.repeating
    }
}
