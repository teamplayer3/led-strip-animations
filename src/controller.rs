use core::cell::RefCell;

use alloc::{borrow::ToOwned, boxed::Box, rc::Rc, vec::Vec};

use crate::{
    animation::{Animation, TimedAnimation, TimedAnimationAt},
    processing::{Processor, SingleAnimationProcessor, TimelineProcessor},
    strip::Strip,
    timeline::{Tick, Ticks, Timeline},
};

pub enum StartingPoint {
    Relative(Ticks),
    Absolute(Tick),
    Now,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct AnimationHandle(u32);

impl AnimationHandle {
    fn new() -> Self {
        static mut ID: u32 = 0;
        unsafe {
            let act_id = ID;
            ID += 1;
            Self(act_id)
        }
    }
}

struct Entry<P> {
    // start_time: Timestamp,
    processor: P,
    handle: AnimationHandle,
}

pub struct AnimationController<S> {
    processors: Vec<Entry<Box<dyn Processor>>>,
    current_tick: Tick,
    // last_time: Timestamp,
    strip: Rc<RefCell<S>>,
}

impl<S> AnimationController<S> {
    pub fn new(strip: Rc<RefCell<S>>) -> Self {
        Self {
            processors: Vec::new(),
            current_tick: 0,
            // last_time: 0,
            strip,
        }
    }

    pub fn queue_timeline<T, A>(&mut self, timeline: T, _at_time: StartingPoint) -> AnimationHandle
    where
        A: TimedAnimationAt<S> + 'static,
        T: Timeline<S, A> + 'static,
        S: Strip + 'static,
    {
        // let time_offset = match at_time {
        //     Timepoint::Absolute(t) => t,
        //     Timepoint::Relative(t) => self.last_time + t,
        //     Timepoint::Now => self.last_time + 1,
        // };
        let handle = AnimationHandle::new();
        self.processors.push(Entry {
            processor: Box::new(TimelineProcessor::new(timeline, self.strip.to_owned())),
            // start_time: time_offset,
            handle: handle.clone(),
        });

        handle
    }

    pub fn queue_animation<A>(&mut self, animation: A, _at_time: StartingPoint) -> AnimationHandle
    where
        A: Animation<S> + 'static,
        S: Strip + 'static,
    {
        let handle = AnimationHandle::new();
        self.processors.push(Entry {
            processor: Box::new(SingleAnimationProcessor::new(
                TimedAnimation::new(self.current_tick, animation),
                self.strip.clone(),
            )),
            handle: handle.clone(),
        });

        handle
    }

    pub fn stop_animation(&mut self, animation_handle: AnimationHandle) {
        self.remove_processor(|e| e.handle == animation_handle);
    }

    pub fn update(&mut self) {
        for e in self.processors.iter_mut() {
            e.processor.update(self.current_tick);
        }

        self.remove_processor(|e| e.processor.has_no_work());
        self.current_tick += 1;
    }

    pub fn has_no_work(&self) -> bool {
        self.processors.len() == 0
    }

    fn remove_processor(&mut self, predicate: impl Fn(&Entry<Box<dyn Processor>>) -> bool) {
        self.processors
            .iter()
            .enumerate()
            .find_map(|e| if predicate(e.1) { Some(e.0) } else { None })
            .map(|i| {
                self.processors.remove(i);
            });
    }
}
