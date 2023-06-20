use crate::{
    animation::{Animation, BoxedAnimation, TimedAnimationAt, TimedAt},
    strip::Strip,
};

use super::animation::TimedAnimation;

pub type Tick = u32;
pub type Ticks = u32;

pub trait Timeline<S, A>
where
    A: Animation<S> + TimedAt + 'static,
    S: Strip + 'static,
{
    type Iter<'a>: Iterator<Item = &'a dyn TimedAnimationAt<S>>
    where
        Self: 'a;

    fn get_current_entries(&self, current_tick: Tick) -> Self::Iter<'_>;
    fn has_finished(&self, current_tick: Tick) -> bool;
    fn should_repeat(&self) -> bool;
}

#[derive(Default)]
pub struct DynTimelineBuilder<S> {
    animations: alloc::vec::Vec<TimedAnimation<BoxedAnimation<S>, S>>,
    repeating: bool,
}

impl<S> DynTimelineBuilder<S>
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
        A: crate::animation::Animation<S> + 'static,
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

    pub fn finish(self) -> DynTimeline<S> {
        let mut animations = self.animations;
        animations.sort_by(|a, b| a.0.cmp(&b.0));
        DynTimeline {
            entries: animations,
            repeating: self.repeating,
        }
    }
}

pub struct DynTimeline<S> {
    entries: alloc::vec::Vec<TimedAnimation<crate::animation::BoxedAnimation<S>, S>>,
    repeating: bool,
}

impl<S> DynTimeline<S> {
    pub fn new(repeating: bool) -> Self {
        let entries = { alloc::vec::Vec::new() };
        Self { entries, repeating }
    }
}

pub struct DynTimelineIter<'a, S> {
    s: &'a [TimedAnimation<crate::animation::BoxedAnimation<S>, S>],
    act_index: usize,
    within_tick: Tick,
}

impl<'a, S> DynTimelineIter<'a, S> {
    pub(crate) fn new(
        animations: &'a alloc::vec::Vec<TimedAnimation<crate::animation::BoxedAnimation<S>, S>>,
        within_tick: Tick,
    ) -> Self {
        Self {
            s: animations.as_slice(),
            act_index: 0,
            within_tick,
        }
    }
}

impl<'a, S> Iterator for DynTimelineIter<'a, S>
where
    S: Strip,
{
    type Item = &'a dyn TimedAnimationAt<S>;

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
                Some(act_animation as &dyn TimedAnimationAt<S>)
            } else {
                None
            }
        })
    }
}

impl<S, A> Timeline<S, A> for DynTimeline<S>
where
    A: Animation<S> + TimedAt + 'static,
    S: Strip + 'static,
{
    type Iter<'a> = DynTimelineIter<'a, S>;

    fn get_current_entries(&self, current_tick: Tick) -> Self::Iter<'_> {
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
