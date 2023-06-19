use core::{cell::RefCell, fmt::Debug, marker::PhantomData, ops::Deref};

use alloc::{boxed::Box, rc::Rc};

use crate::{
    color::{HSVColor, LedColoring},
    strip::Strip,
};

use super::timeline::{Tick, Ticks};

mod running_light;
mod static_animation;

pub use running_light::{AnimationPart, AnimationSymmetry, AnimationType, RunningLight};
pub use static_animation::StaticAnimation;

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

#[cfg(test)]
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

#[cfg(feature = "nightly")]
pub trait Animation<S, A: Allocator = Global>
where
    S: Strip,
{
    fn animate(
        &self,
        current_tick: Tick,
        strip: &S,
        animation_meta: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>, A>;

    fn duration(&self) -> Ticks;
}

#[cfg(not(feature = "nightly"))]
pub trait Animation<S>
where
    S: Strip,
{
    fn animate(
        &self,
        current_tick: Tick,
        strip: Rc<RefCell<S>>,
        animation_meta: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>>;

    fn duration(&self) -> Ticks;
}

#[cfg(feature = "nightly")]
pub type BoxedAnimation<A> = Box<dyn Animation<A>>;

#[cfg(not(feature = "nightly"))]
pub type BoxedAnimation<S> = Box<dyn Animation<S>>;

#[cfg(not(feature = "nightly"))]
impl<S> Animation<S> for BoxedAnimation<S>
where
    S: Strip,
{
    fn animate(
        &self,
        current_tick: Tick,
        strip: Rc<RefCell<S>>,
        animation_meta: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>> {
        self.deref().animate(current_tick, strip, animation_meta)
    }

    fn duration(&self) -> Ticks {
        self.deref().duration()
    }
}

pub trait TimedAt {
    fn at_tick(&self) -> Tick;
}

pub trait TimedAnimationAt<S: Strip>: TimedAt + Animation<S> {}

impl<S, T> TimedAnimationAt<S> for T
where
    T: Animation<S> + TimedAt,
    S: Strip,
{
}

pub struct TimedAnimation<A, S>(pub Tick, pub A, PhantomData<S>);

impl<A, S> TimedAnimation<A, S> {
    pub fn new(at_tick: Tick, animation: A) -> Self {
        Self(at_tick, animation, PhantomData::default())
    }
}

impl<A, S> TimedAnimation<A, S>
where
    A: Animation<S>,
    S: Strip,
{
    pub fn animation_duration(&self) -> Ticks {
        self.1.duration()
    }
}

impl<A, S> Animation<S> for TimedAnimation<A, S>
where
    S: Strip,
    A: Animation<S>,
{
    fn animate(
        &self,
        current_tick: Tick,
        strip: Rc<RefCell<S>>,
        animation_meta: &AnimationMeta,
    ) -> Box<dyn Iterator<Item = LedColoring<HSVColor>>> {
        self.1.animate(current_tick, strip, animation_meta)
    }

    fn duration(&self) -> Ticks {
        self.1.duration()
    }
}

impl<A, S> TimedAt for TimedAnimation<A, S> {
    fn at_tick(&self) -> Tick {
        self.0
    }
}

impl<A, S> Debug for TimedAnimation<A, S>
where
    A: Animation<S>,
    S: Strip,
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
