use keyframe::{ease_with_scaled_time, functions, CanTween};

use super::timeline::{Tick, Ticks};

#[derive(Debug, Clone, Copy)]
pub enum Curve {
    Linear,
    Step,
    EaseIn,
    EaseOut,
    EaseInOut,
}

pub(crate) fn calculate_with_curve<H: CanTween + Copy>(
    curve: &Curve,
    duration: Ticks,
    from: &H,
    to: &H,
    current_tick: Tick,
) -> H {
    let from = from.clone();
    let to = to.clone();
    match curve {
        Curve::Linear => ease_with_scaled_time(
            functions::Linear,
            from,
            to,
            current_tick as f32,
            duration as f32,
        ),

        Curve::Step => ease_with_scaled_time(
            functions::Step,
            from,
            to,
            current_tick as f32,
            duration as f32,
        ),

        Curve::EaseIn => ease_with_scaled_time(
            functions::EaseIn,
            from,
            to,
            current_tick as f32,
            duration as f32,
        ),

        Curve::EaseInOut => ease_with_scaled_time(
            functions::EaseInOut,
            from,
            to,
            current_tick as f32,
            duration as f32,
        ),
        Curve::EaseOut => ease_with_scaled_time(
            functions::EaseOut,
            from,
            to,
            current_tick as f32,
            duration as f32,
        ),
    }
}
