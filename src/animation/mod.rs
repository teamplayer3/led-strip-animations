use core::{fmt::Debug, ops::Deref};

use crate::{
    color::{HSVColor, LedColoring},
    strip::Strip,
};

use super::timeline::{Tick, Ticks};

mod running_light;
mod static_animation;

#[cfg(test)]
mod testing;

pub enum IterationState {
    First {
        remaining_iterations: u32,
    },
    Looping {
        iteration_index: u32,
        remaining_iterations: u32,
    },
    Last {
        iteration_index: u32,
    },
    Single,
}

impl IterationState {
    pub(super) fn new(iteration_index: u32, remaining_iterations: u32) -> Self {
        if iteration_index == 0 && remaining_iterations == 0 {
            Self::Single
        } else if iteration_index == 0 {
            Self::First {
                remaining_iterations,
            }
        } else if remaining_iterations == 0 {
            Self::Last { iteration_index }
        } else {
            Self::Looping {
                iteration_index,
                remaining_iterations,
            }
        }
    }

    pub(super) fn single() -> Self {
        Self::new(0, 0)
    }
}

pub struct AnimationMeta {
    pub iteration_state: IterationState,
}

impl AnimationMeta {
    pub fn new(iteration_state: IterationState) -> Self {
        Self { iteration_state }
    }
}

pub trait Animation {
    type Strip: Strip;
    type Iter: Iterator<Item = LedColoring<HSVColor>>;

    fn animate(
        &self,
        current_tick: Tick,
        strip: &Self::Strip,
        animation_meta: &AnimationMeta,
    ) -> Self::Iter;

    fn duration(&self) -> Ticks;
}

#[cfg(feature = "alloc")]
pub type BoxedAnimation<S, I> = alloc::boxed::Box<dyn Animation<Strip = S, Iter = I>>;

#[cfg(feature = "alloc")]
impl<S, I> Animation for BoxedAnimation<S, I>
where
    S: Strip,
    I: Iterator<Item = LedColoring<HSVColor>>,
{
    type Strip = S;
    type Iter = I;

    fn animate(
        &self,
        current_tick: Tick,
        strip: &Self::Strip,
        animation_meta: &AnimationMeta,
    ) -> Self::Iter {
        use core::ops::Deref;
        self.deref().animate(current_tick, strip, animation_meta)
    }

    fn duration(&self) -> Ticks {
        use core::ops::Deref;
        self.deref().duration()
    }
}

pub struct TimedAnimation<A>(pub Tick, pub A);

impl<A> TimedAnimation<A> {
    pub fn new(at_tick: Tick, animation: A) -> Self {
        Self(at_tick, animation)
    }
}

impl<A> TimedAnimation<A>
where
    A: Animation,
{
    pub fn animation_duration(&self) -> Ticks {
        self.1.duration()
    }
}

impl<A: Sized> TimedAnimation<alloc::boxed::Box<A>> {
    pub fn deref_inner<'a>(&'a self) -> TimedAnimation<&'a A> {
        TimedAnimation(self.0, self.1.deref())
    }
}

impl<A> Debug for TimedAnimation<A>
where
    A: Animation,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TimedAnimation")
            .field(&self.0)
            .field(&self.1.duration())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FromColoring {
    Dynamic,
    Fixed(HSVColor),
}
