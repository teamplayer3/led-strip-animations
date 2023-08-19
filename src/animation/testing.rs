use core::cell::RefCell;

use alloc::rc::Rc;
use assert_matches::assert_matches;

use crate::{color::HSVColor, indexing::LedId, strip::Strip, timeline::Tick};

use super::{Animation, AnimationMeta, IterationState};

pub enum Iterations {
    // Infinite,
    Single,
    // Some(u32),
}

impl Iterations {
    fn as_iteration_state(&self, _iteration_index: u32) -> IterationState {
        match self {
            Iterations::Single => IterationState::single(),
            // Iterations::Infinite => IterationState::new(iteration_index, u32::MAX),
            // Iterations::Some(n) => IterationState::new(iteration_index, n - iteration_index - 1),
        }
    }
}

pub struct AnimationTester<A, S>
where
    A: Animation<S>,
    S: Strip,
{
    animation: A,
    iterate: Iterations,
    iteration_index: u32,
    strip: Rc<RefCell<S>>,
}

impl<A, S> AnimationTester<A, S>
where
    A: Animation<S>,
    S: Strip,
{
    pub fn new(animation: A, iterate: Iterations, strip: Rc<RefCell<S>>) -> Self {
        Self {
            animation,
            iterate,
            iteration_index: 0,
            strip,
        }
    }
}

impl<S, A> AnimationTester<A, S>
where
    A: Animation<S>,
    S: Strip,
{
    pub fn print_state(&mut self, tick: Tick) {
        extern crate std;
        let animation_meta =
            AnimationMeta::new(self.iterate.as_iteration_state(self.iteration_index));
        let mut led_iter = self
            .animation
            .animate(tick, self.strip.clone(), &animation_meta);
        while let Some(led_colored) = led_iter.next() {
            std::println!("{:?}", led_colored);
        }
    }

    pub fn assert_state<T: IntoIterator<Item = (LedId, HSVColor)>>(
        &mut self,
        tick: Tick,
        should_state: T,
    ) {
        let animation_meta =
            AnimationMeta::new(self.iterate.as_iteration_state(self.iteration_index));
        let mut led_iter = self
            .animation
            .animate(tick, self.strip.clone(), &animation_meta);
        for (led_id, color) in should_state {
            let next_led = led_iter.next();
            match next_led {
                None => panic!("LED iterate has not the same length as should_state"),
                Some(led_colored) => {
                    assert_eq!(
                        led_colored.led, led_id,
                        "LED has wrong id: {:?}, should be: {:?}",
                        led_colored.led, led_id
                    );
                    assert_eq!(
                        led_colored.color, color,
                        "LED with id {:?} should have color: {:?} but has: {:?}",
                        led_id, color, led_colored.color,
                    );
                }
            }
        }
        assert_matches!(
            led_iter.next(),
            None,
            "LED iterate has not the same length as should_state"
        )
    }
}
