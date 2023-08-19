use crate::{
    color::{HSVColor, Spectrum, TransparentColor},
    curve::{calculate_with_curve, Curve},
    indexing::LedId,
    util::range_map,
};

pub struct Progress {
    pub current_led_id: LedId,
    pub len: u16,
}

impl Progress {
    pub fn new(current_led_id: LedId, len: u16) -> Self {
        Self {
            current_led_id,
            len,
        }
    }

    pub fn progress(&self) -> f32 {
        self.current_led_id as f32 / (self.len as f32 - 1.0)
    }
}

pub trait Pattern {
    type Color;

    fn color_at(&self, progress: Progress) -> TransparentColor<Self::Color>;
}

impl<S, C> Pattern for S
where
    S: Spectrum<Color = C>,
{
    type Color = C;

    fn color_at(&self, progress: Progress) -> TransparentColor<Self::Color> {
        self.color_at(progress.progress())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HillPattern<S> {
    peak_len: u16,
    spectrum: S,
    curve: Curve,
}

impl<S> HillPattern<S> {
    pub fn new(peak_len: u16, spectrum: S, curve: Curve) -> Self {
        Self {
            peak_len,
            spectrum,
            curve,
        }
    }
}

impl<S> Pattern for HillPattern<S>
where
    S: Spectrum<Color = HSVColor>,
{
    type Color = HSVColor;

    fn color_at(&self, progress: Progress) -> TransparentColor<Self::Color> {
        let fade_len = (progress.len - self.peak_len) / 2;

        let peak_color = self.spectrum.color_at(0.5);

        if progress.current_led_id < fade_len {
            let current_color = self.spectrum.color_at(progress.progress());

            let transparency = calculate_with_curve(
                &self.curve,
                fade_len as u32,
                &1.0,
                &0.0,
                progress.current_led_id as u32,
            );

            let transparency = range_map(transparency, 0.0, 1.0, peak_color.transparency, 1.0);

            TransparentColor::new(current_color.color, transparency)
        } else if progress.current_led_id >= progress.len - fade_len {
            let current_color = self.spectrum.color_at(progress.progress());

            let transparency = calculate_with_curve(
                &self.curve,
                fade_len as u32,
                &0.0,
                &1.0,
                (progress.current_led_id + 1 - self.peak_len - fade_len) as u32,
            );

            let transparency = range_map(transparency, 0.0, 1.0, peak_color.transparency, 1.0);

            TransparentColor::new(current_color.color, transparency)
        } else {
            peak_color
        }
    }
}

#[cfg(test)]
mod test {

    use crate::color::{RainbowSpectrum, SpectrumExt};

    use super::*;

    struct PatternTester<P>
    where
        P: Pattern,
    {
        pattern: P,
        pattern_len: u16,
    }

    impl<P> PatternTester<P>
    where
        P: Pattern<Color = HSVColor>,
    {
        fn new(pattern: P, pattern_len: u16) -> Self {
            Self {
                pattern,
                pattern_len,
            }
        }

        fn assert(&self, pattern_assert: &[TransparentColor<HSVColor>]) {
            for (tick, pat) in pattern_assert.into_iter().enumerate() {
                let progress = Progress::new(tick as LedId, self.pattern_len);
                let color = self.pattern.color_at(progress);
                assert_eq!(
                    color.map(|_, _, t| *t = (*t * 100.0).round() / 100.0),
                    *pat,
                    "tick: {}",
                    tick
                );
            }
        }
    }

    #[test]
    fn test_hill_pattern_peak_len_4_single_color() {
        let pattern_len = 10;
        let peak_color = HSVColor::new(0, 100, 100);
        let pattern = HillPattern::new(4, peak_color, Curve::Linear);

        let pattern_assert = [
            peak_color.with_transparency(1.0),
            peak_color.with_transparency(0.67),
            peak_color.with_transparency(0.33),
            peak_color.with_transparency(0.0),
            peak_color.with_transparency(0.0),
            peak_color.with_transparency(0.0),
            peak_color.with_transparency(0.0),
            peak_color.with_transparency(0.33),
            peak_color.with_transparency(0.67),
            peak_color.with_transparency(1.0),
        ];

        let tester = PatternTester::new(pattern, pattern_len);

        tester.assert(&pattern_assert);
    }

    #[test]
    fn test_hill_pattern_peak_len_2_rainbow_spectrum() {
        let pattern_len = 10;
        let spectrum =
            RainbowSpectrum::new(HSVColor::new(0, 100, 100), HSVColor::new(100, 100, 100));
        let pattern = HillPattern::new(2, spectrum, Curve::Linear);

        let pattern_assert = [
            HSVColor::new(0, 100, 100).with_transparency(1.0),
            HSVColor::new(11, 100, 100).with_transparency(0.75),
            HSVColor::new(22, 100, 100).with_transparency(0.5),
            HSVColor::new(33, 100, 100).with_transparency(0.25),
            HSVColor::new(50, 100, 100).with_transparency(0.0),
            HSVColor::new(50, 100, 100).with_transparency(0.0),
            HSVColor::new(66, 100, 100).with_transparency(0.25),
            HSVColor::new(77, 100, 100).with_transparency(0.5),
            HSVColor::new(88, 100, 100).with_transparency(0.75),
            HSVColor::new(100, 100, 100).with_transparency(1.0),
        ];

        let tester = PatternTester::new(pattern, pattern_len);

        tester.assert(&pattern_assert);
    }
}
