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
    let from = *from;
    let to = *to;
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

pub(crate) fn calculate_with_curve_percentage<H: CanTween + Copy>(
    curve: &Curve,
    from: &H,
    to: &H,
    percentage: f32,
) -> H {
    calculate_with_curve(curve, 1000, from, to, (percentage * 1000f32) as Tick)
}
