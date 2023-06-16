use assert_matches::assert_matches;

use crate::{color::HSVColor, indexing::LedId, strip::Strip, timeline::Tick};

use super::{Animation, AnimationMeta, IterationState};

pub enum Iterations {
    Infinite,
    Single,
    Some(u32),
}

impl Iterations {
    fn as_iteration_state(&self, iteration_index: u32) -> IterationState {
        match self {
            Iterations::Single => IterationState::single(),
            Iterations::Infinite => IterationState::new(iteration_index, u32::MAX),
            Iterations::Some(n) => IterationState::new(iteration_index, n - iteration_index - 1),
        }
    }
}

pub struct AnimationTester<'a, A, L>
where
    A: Animation,
{
    animation: A,
    iterate: Iterations,
    iteration_index: u32,
    strip: &'a mut L,
}

impl<'a, A, L> AnimationTester<'a, A, L>
where
    A: Animation,
{
    pub fn new(animation: A, iterate: Iterations, strip: &'a mut L) -> Self {
        Self {
            animation,
            iterate,
            iteration_index: 0,
            strip,
        }
    }
}

impl<'a, A, L> AnimationTester<'a, A, L>
where
    A: Animation<Strip = L>,
    L: Strip,
{
    pub fn assert_state<I: IntoIterator<Item = (LedId, HSVColor)>>(
        &mut self,
        tick: Tick,
        should_state: I,
    ) {
        let animation_meta =
            AnimationMeta::new(self.iterate.as_iteration_state(self.iteration_index));
        let mut led_iter = self.animation.animate(tick, &self.strip, &animation_meta);
        for (led_id, color) in should_state {
            assert_matches!(led_iter.next(), Some(led_colored) if led_colored.led == led_id && led_colored.color == color)
        }
        assert_matches!(led_iter.next(), None)
    }
}
