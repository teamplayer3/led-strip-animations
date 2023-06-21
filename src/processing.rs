use alloc::rc::Rc;
use core::{cell::RefCell, marker::PhantomData};

use crate::{
    animation::{IterationState, TimedAnimationAt},
    strip::Strip,
};

use super::{
    animation::AnimationMeta,
    timeline::{Tick, Ticks, Timeline},
};

pub trait Processor {
    fn update(&mut self, current_tick: Tick);
    fn has_no_work(&self) -> bool;
}

pub struct TimelineProcessor<A, T, S> {
    timeline: T,
    strip: Rc<RefCell<S>>,
    no_work: bool,
    tick_offset: Ticks,
    iteration_index: u32,
    _animation: PhantomData<A>,
}

impl<A, T, S> TimelineProcessor<A, T, S> {
    pub fn new(timeline: T, strip: Rc<RefCell<S>>) -> Self {
        Self {
            timeline,
            strip,
            no_work: false,
            tick_offset: 0,
            iteration_index: 0,
            _animation: PhantomData::default(),
        }
    }
}

impl<A, T, S> Processor for TimelineProcessor<A, T, S>
where
    A: TimedAnimationAt<S> + 'static,
    T: Timeline<S, A>,
    S: Strip + 'static,
{
    fn update(&mut self, current_tick: Tick) {
        if self.timeline.has_finished(current_tick - self.tick_offset) {
            if self.timeline.should_repeat() {
                self.tick_offset = current_tick;
                self.iteration_index += 1;
            } else {
                self.no_work = true;
            }
        }

        let tick = current_tick - self.tick_offset;
        let animations = self.timeline.get_current_entries(tick);

        // TODO: make max iteration count variable iteration count
        let iteration_state = IterationState::new(self.iteration_index, u32::MAX);
        let animation_meta = AnimationMeta::new(iteration_state);

        for anim in animations {
            let start_time = anim.at_tick();
            let animation_step =
                anim.animate(tick - start_time, self.strip.clone(), &animation_meta);

            for coloring in animation_step {
                self.strip
                    .borrow_mut()
                    .set_led_to_color(coloring.led, &coloring.color.into())
            }
        }
    }

    fn has_no_work(&self) -> bool {
        self.no_work
    }
}

pub struct SingleAnimationProcessor<A, S> {
    animation: A,
    strip: Rc<RefCell<S>>,
    has_finished: bool,
}

impl<A, S> SingleAnimationProcessor<A, S> {
    pub fn new(animation: A, strip: Rc<RefCell<S>>) -> Self {
        Self {
            animation,
            strip,
            has_finished: false,
        }
    }
}

impl<A, S> Processor for SingleAnimationProcessor<A, S>
where
    A: TimedAnimationAt<S> + 'static,
    S: Strip + 'static,
{
    fn update(&mut self, current_tick: Tick) {
        let start = self.animation.at_tick();
        if start + self.animation.duration() > current_tick {
            self.has_finished = true;
            return;
        }

        let animation_step = self.animation.animate(
            current_tick - start,
            self.strip.clone(),
            &AnimationMeta::new(IterationState::single()),
        );

        for coloring in animation_step {
            self.strip
                .borrow_mut()
                .set_led_to_color(coloring.led, &coloring.color.into())
        }
    }

    fn has_no_work(&self) -> bool {
        self.has_finished
    }
}
