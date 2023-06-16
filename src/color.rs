use core::{
    cmp::max,
    ops::{Add, Index, IndexMut, Mul, Sub},
};

use keyframe::CanTween;
use num_traits::Zero;
use rgb::RGB8;

use crate::{
    indexing::LedId,
    util::{max_3, min_3, wrap_on},
};

const MAX_RGB_VALUE: u8 = 255;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    pub const fn init(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn as_raw(&self) -> [u8; 4] {
        [self.r, self.g, self.b, 0]
    }

    pub const fn as_raw_bgr(&self) -> [u8; 4] {
        [self.b, self.g, self.r, 0]
    }

    pub const fn off() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    pub const fn red() -> Self {
        Self {
            r: MAX_RGB_VALUE,
            g: 0,
            b: 0,
        }
    }

    pub const fn green() -> Self {
        Self {
            r: 0,
            g: MAX_RGB_VALUE,
            b: 0,
        }
    }

    pub const fn blue() -> Self {
        Self {
            r: 0,
            g: 0,
            b: MAX_RGB_VALUE,
        }
    }

    pub const fn white() -> Self {
        Self {
            r: MAX_RGB_VALUE,
            g: MAX_RGB_VALUE,
            b: MAX_RGB_VALUE,
        }
    }
}

impl From<RGB8> for Color {
    fn from(rgb8: RGB8) -> Self {
        Color::init(rgb8.r, rgb8.g, rgb8.b)
    }
}

impl From<Color> for RGB8 {
    fn from(c: Color) -> Self {
        RGB8::new(c.r, c.g, c.b)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(val: (u8, u8, u8)) -> Self {
        Color::init(val.0, val.1, val.2)
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Color {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
        }
    }
}

impl<F> Mul<F> for Color
where
    F: num_traits::Float,
{
    type Output = Self;

    fn mul(self, rhs: F) -> Self::Output {
        let mul = rhs.to_f32().expect("could not parse float");
        Color {
            r: (self.r as f32 * mul) as u8,
            g: (self.g as f32 * mul) as u8,
            b: (self.b as f32 * mul) as u8,
        }
    }
}

impl CanTween for Color {
    fn ease(from: Self, to: Self, time: impl keyframe::num_traits::Float) -> Self {
        from + (to - from) * time
    }
}

impl From<[u8; 3]> for Color {
    fn from(v: [u8; 3]) -> Self {
        Self::init(v[0], v[1], v[2])
    }
}

impl Index<u8> for &Color {
    type Output = u8;

    fn index(&self, index: u8) -> &Self::Output {
        assert!(index < 3);
        match index {
            0 => &self.r,
            1 => &self.g,
            2 => &self.b,
            i => panic!("index {} describes not a color value", i),
        }
    }
}

impl Index<u8> for Color {
    type Output = u8;

    fn index(&self, index: u8) -> &Self::Output {
        assert!(index < 3);
        match index {
            0 => &self.r,
            1 => &self.g,
            2 => &self.b,
            i => panic!("index {} describes not a color value", i),
        }
    }
}

impl IndexMut<u8> for Color {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        assert!(index < 3);
        match index {
            0 => &mut self.r,
            1 => &mut self.g,
            2 => &mut self.b,
            i => panic!("index {} describes not a color value", i),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HSVColor {
    pub h: u16,
    pub s: u8,
    pub v: u8,
}

impl HSVColor {
    pub fn new(h: u16, s: u8, v: u8) -> Self {
        assert!(h <= 360, "hue must be in range 0..=360");
        assert!(s <= 100, "saturation must be in range 0..=100");
        assert!(v <= 100, "value must be in range 0..=100");
        Self { h, s, v }
    }

    pub fn off_from_color(color: Color) -> Self {
        let hsv = Self::from(color);
        Self {
            h: hsv.h,
            s: hsv.s,
            v: 0,
        }
    }

    pub fn darken(self, amount: u8) -> Self {
        let new_v = self.v.sub(amount);
        Self {
            h: self.h,
            s: self.s,
            v: new_v,
        }
    }

    pub fn brighten(self, amount: u8) -> Self {
        let new_v = max(self.v.add(amount), 100);
        Self {
            h: self.h,
            s: self.s,
            v: new_v,
        }
    }
}

impl From<Color> for HSVColor {
    fn from(c: Color) -> Self {
        let (h, s, v) = rgb_to_hsv(c.r, c.g, c.b);
        Self { h, s, v }
    }
}

impl From<HSVColor> for Color {
    fn from(c: HSVColor) -> Self {
        let (r, g, b) = hsv_to_rgb(c.h, c.s, c.v);
        Self { r, g, b }
    }
}

impl CanTween for HSVColor {
    fn ease(from: Self, to: Self, time: impl num_traits::Float) -> Self {
        let off_on_fade = from.v == 0 && to.v > 0;
        let on_off_fade = to.v == 0 && from.v > 0;
        HSVColor::new(
            if off_on_fade {
                to.h
            } else if on_off_fade {
                from.h
            } else {
                wrap_on(
                    (from.h as i16
                        + time
                            .mul(num_traits::NumCast::from(to.h as i16 - from.h as i16).unwrap())
                            .to_i16()
                            .unwrap()) as u16,
                    360,
                )
            },
            if off_on_fade {
                to.s
            } else if on_off_fade {
                from.s
            } else {
                wrap_on(
                    (from.s as i8
                        + time
                            .mul(num_traits::NumCast::from(to.s as i8 - from.s as i8).unwrap())
                            .to_i8()
                            .unwrap()) as u8,
                    100,
                )
            },
            wrap_on(
                (from.v as i8
                    + time
                        .mul(num_traits::NumCast::from(to.v as i8 - from.v as i8).unwrap())
                        .to_i8()
                        .unwrap()) as u8,
                100,
            ),
        )
    }
}

#[derive(Debug)]
pub struct LedColoring<C> {
    pub led: LedId,
    pub color: C,
}

impl<C> LedColoring<C> {
    pub fn new(led: LedId, color: C) -> Self {
        Self { led, color }
    }
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r = (r as f32) / 255f32;
    let g = (g as f32) / 255f32;
    let b = (b as f32) / 255f32;
    let c_max = max_3(r, g, b);
    let c_min = min_3(r, g, b);
    let diff = c_max - c_min;

    let h = 60f32
        * if diff.is_zero() {
            0f32
        } else if c_max.eq(&r) {
            ((g - b) / diff) % 6f32
        } else if c_max.eq(&g) {
            ((b - r) / diff) + 2f32
        } else {
            ((r - g) / diff) + 4f32
        };

    let s = if c_max.is_zero() { 0f32 } else { diff / c_max };

    let v = c_max;

    (h as u16, (s * 100f32) as u8, (v * 100f32) as u8)
}

fn hsv_to_rgb(h: u16, s: u8, v: u8) -> (u8, u8, u8) {
    let s = s as f32 / 100f32;
    let v = v as f32 / 100f32;
    let c = v * s;
    let x = c * (1f32 - ((h as f32 / 60f32) % 2f32 - 1f32).abs());
    let m = v - c;

    let (r, g, b) = if h < 60 {
        (c, x, 0f32)
    } else if h < 120 {
        (x, c, 0f32)
    } else if h < 180 {
        (0f32, c, x)
    } else if h < 240 {
        (0f32, x, c)
    } else if h < 300 {
        (x, 0f32, c)
    } else {
        (c, 0f32, x)
    };

    // only nightly
    // let (r, g, b) = match h {
    //     0..60 => (c, x, 0f32),
    //     60..120 => (x, c, 0f32),
    //     120..180 => (0f32, c, x),
    //     180..240 => (0f32, x, c),
    //     240..300 => (x, 0f32, c),
    //     300..=360 => (c, 0f32, x),
    //     _ => panic!("hue must be 0 < h <= 360"),
    // };

    let r = ((r + m) * 255f32) as u8;
    let g = ((g + m) * 255f32) as u8;
    let b = ((b + m) * 255f32) as u8;

    (r, g, b)
}

#[cfg(test)]
mod test {
    use keyframe::{ease_with_scaled_time, functions};

    use super::*;

    #[test]
    fn max_of_three_values_a() {
        let a = 25.5;
        let b = 15.1;
        let c = 19.2;

        let max = max_3(a, b, c);
        assert_eq!(max, a)
    }

    #[test]
    fn max_of_three_values_b() {
        let a = 13.5;
        let b = 25.1;
        let c = 19.2;

        let max = max_3(a, b, c);
        assert_eq!(max, b)
    }

    #[test]
    fn max_of_three_values_c() {
        let a = 13.5;
        let b = 15.1;
        let c = 19.2;

        let max = max_3(a, b, c);
        assert_eq!(max, c)
    }

    #[test]
    fn min_of_three_values_a() {
        let a = 13.5;
        let b = 15.1;
        let c = 19.2;

        let min = min_3(a, b, c);
        assert_eq!(min, a)
    }

    #[test]
    fn min_of_three_values_b() {
        let a = 13.5;
        let b = 11.1;
        let c = 19.2;

        let min = min_3(a, b, c);
        assert_eq!(min, b)
    }

    #[test]
    fn min_of_three_values_c() {
        let a = 13.5;
        let b = 15.1;
        let c = 11.2;

        let min = min_3(a, b, c);
        assert_eq!(min, c)
    }

    #[test]
    fn min_of_three_values_special() {
        let a = 0.0;
        let b = 1.0;
        let c = 0.0;

        let min = min_3(a, b, c);
        assert_eq!(min, a)
    }

    #[test]
    fn rgb_to_hsv_white() {
        let r = 255;
        let g = 255;
        let b = 255;

        let hsv = rgb_to_hsv(r, g, b);
        assert_eq!(hsv, (0, 0, 100))
    }

    #[test]
    fn rgb_to_hsv_red() {
        let r = 255;
        let g = 0;
        let b = 0;

        let hsv = rgb_to_hsv(r, g, b);
        assert_eq!(hsv, (0, 100, 100))
    }

    #[test]
    fn rgb_to_hsv_green() {
        let r = 0;
        let g = 255;
        let b = 0;

        let hsv = rgb_to_hsv(r, g, b);
        assert_eq!(hsv, (120, 100, 100))
    }

    #[test]
    fn rgb_to_hsv_blue() {
        let r = 0;
        let g = 0;
        let b = 255;

        let hsv = rgb_to_hsv(r, g, b);
        assert_eq!(hsv, (240, 100, 100))
    }

    #[test]
    fn hsv_to_rgb_white() {
        let h = 0;
        let s = 0;
        let v = 100;

        let rgb = hsv_to_rgb(h, s, v);
        assert_eq!(rgb, (255, 255, 255))
    }

    #[test]
    fn hsv_to_rgb_red() {
        let h = 0;
        let s = 100;
        let v = 100;

        let rgb = hsv_to_rgb(h, s, v);
        assert_eq!(rgb, (255, 0, 0))
    }

    #[test]
    fn hsv_to_rgb_green() {
        let h = 120;
        let s = 100;
        let v = 100;

        let rgb = hsv_to_rgb(h, s, v);
        assert_eq!(rgb, (0, 255, 0))
    }

    #[test]
    fn hsv_to_rgb_blue() {
        let h = 240;
        let s = 100;
        let v = 100;

        let rgb = hsv_to_rgb(h, s, v);
        assert_eq!(rgb, (0, 0, 255))
    }

    #[test]
    fn hsv_to_rgb_360() {
        let h = 360;
        let s = 100;
        let v = 100;

        let rgb = hsv_to_rgb(h, s, v);
        assert_eq!(rgb, (255, 0, 0))
    }

    #[test]
    fn ease_color_off_red_half() {
        let to = HSVColor::from(Color::red());
        let from = HSVColor::off_from_color(Color::red());

        let end = ease_with_scaled_time(functions::Linear, from, to.clone(), 5.0, 10.0);
        let mut half_red = to.clone();
        half_red.v = 50;
        assert_eq!(end, half_red)
    }

    #[test]
    fn ease_color_off_red_steps() {
        let to = HSVColor::new(100, 100, 100);
        let from = HSVColor::new(0, 0, 0);

        let step = ease_with_scaled_time(functions::Linear, from, to.clone(), 0.0, 2.0);
        assert_eq!(step, HSVColor::new(100, 100, 0));

        let step = ease_with_scaled_time(functions::Linear, from, to.clone(), 1.0, 2.0);
        assert_eq!(step, HSVColor::new(100, 100, 50));

        let step = ease_with_scaled_time(functions::Linear, from, to.clone(), 2.0, 2.0);
        assert_eq!(step, HSVColor::new(100, 100, 100));
    }

    #[test]
    fn ease_color_off_red_goal() {
        let to = HSVColor::from(Color::red());
        let from = HSVColor::off_from_color(Color::red());

        let end = ease_with_scaled_time(functions::Linear, from, to.clone(), 10.0, 10.0);
        assert_eq!(to, end)
    }

    #[test]
    fn ease_color_special() {
        let to = HSVColor::new(0, 100, 0);
        let from = HSVColor::new(0, 100, 100);

        let end = ease_with_scaled_time(functions::Linear, from, to.clone(), 1.0, 2.0);
        assert_eq!(HSVColor::new(0, 100, 50), end)
    }
}
