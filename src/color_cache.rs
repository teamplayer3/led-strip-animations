use core::ops::Range;

use alloc::{boxed::Box, vec::Vec};

use crate::{color::HSVColor, indexing::LedId};

#[derive(Debug)]
struct SingleColor {
    ranges: Vec<Range<LedId>>,
    single_led: Vec<LedId>,
    color: HSVColor,
}

impl SingleColor {
    fn new(color: HSVColor) -> Self {
        Self {
            ranges: Default::default(),
            single_led: Default::default(),
            color,
        }
    }

    fn contains_led_id(&self, led_id: LedId) -> bool {
        self.ranges.iter().any(|range| range.contains(&led_id)) || self.single_led.contains(&led_id)
    }

    fn cache_led(&mut self, led_id: LedId) {
        let range = self.ranges.iter_mut().find_map(|range| {
            if range.start.eq(&(led_id + 1)) {
                Some((range, true))
            } else if range.end.eq(&led_id) {
                Some((range, false))
            } else {
                None
            }
        });

        if let Some((range, start)) = range {
            if start {
                range.start = led_id
            } else {
                range.end = led_id + 1
            }
        } else if let Some(range) = self.grouping_single_led(led_id) {
            self.ranges.push(range);
        } else {
            self.single_led.push(led_id);
        }
    }

    fn uncache_led(&mut self, led_id: LedId) -> bool {
        let r = self.ungroup_leds(led_id);
        if let Some(ranges) = r {
            for range in ranges {
                match range.len() {
                    1 => self.single_led.push(range.start),
                    l if l > 1 => self.ranges.push(range),
                    _ => (),
                }
            }
            true
        } else {
            let r = self
                .single_led
                .iter()
                .enumerate()
                .find(|(_, led)| (*led).eq(&led_id));
            if let Some((index, _)) = r {
                let _ = self.single_led.remove(index);
                true
            } else {
                false
            }
        }
    }

    fn cached_size(&self) -> usize {
        let range_cache_size = self
            .ranges
            .iter()
            .map(|range| range.len())
            .reduce(|prev, now| prev + now)
            .unwrap_or(0);
        let single_led_cache_size = self.single_led.len();

        range_cache_size + single_led_cache_size
    }

    fn ungroup_leds(&mut self, led_id: LedId) -> Option<[Range<LedId>; 2]> {
        let r = self
            .ranges
            .iter()
            .enumerate()
            .find(|(_, range)| range.contains(&led_id));
        if let Some((index, range)) = r {
            let ranges = [range.start..led_id, (led_id + 1)..range.end];
            self.ranges.remove(index);
            Some(ranges)
        } else {
            None
        }
    }

    fn grouping_single_led(&mut self, led_id: LedId) -> Option<Range<LedId>> {
        let r = self.single_led.iter().enumerate().find_map(|(index, led)| {
            if led.eq(&(led_id + 1)) {
                Some((index, true))
            } else if led_id > 0 && led.eq(&(led_id - 1)) {
                Some((index, false))
            } else {
                None
            }
        });
        if let Some((index, start)) = r {
            let with_led_id = self.single_led.remove(index);
            if start {
                Some(led_id..(with_led_id + 1))
            } else {
                Some(with_led_id..(led_id + 1))
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ColorCache {
    single_color_cache: Option<Box<SingleColor>>,
}

impl ColorCache {
    pub fn new() -> Self {
        Self {
            single_color_cache: None,
        }
    }
}

impl ColorCache {
    /// if color for led id already cached it returns
    /// the color which is already cached as a `Some`.
    pub fn cache_color(&mut self, led_id: LedId, color: &HSVColor) -> Option<HSVColor> {
        self.init(color);
        if let Some(cache) = self.single_color_cache.as_mut() {
            if cache.contains_led_id(led_id) {
                Some(cache.color)
            } else {
                cache.cache_led(led_id);
                None
            }
        } else {
            unreachable!()
        }
    }

    pub fn load_color(&self, led_id: LedId) -> Option<HSVColor> {
        if let Some(cache) = self.single_color_cache.as_ref() {
            if cache.contains_led_id(led_id) {
                Some(cache.color)
            } else {
                None
            }
        } else {
            unreachable!()
        }
    }

    pub fn remove_cache(&mut self, led_id: LedId) -> Option<HSVColor> {
        if let Some(cache) = self.single_color_cache.as_mut() {
            let removed_some_cache = cache.uncache_led(led_id);
            if removed_some_cache {
                Some(cache.color)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cache_size(&self) -> usize {
        if let Some(cache) = self.single_color_cache.as_ref() {
            cache.cached_size()
        } else {
            0
        }
    }

    // fn change_cache_strategy(&mut self) {}

    fn init(&mut self, color: &HSVColor) {
        match self.single_color_cache {
            None => self.single_color_cache = Some(Box::new(SingleColor::new(*color))),
            Some(_) => (),
        }
    }
}

#[cfg(test)]
mod test {
    use core::{mem::size_of_val, ops::Deref};

    use assert_matches::assert_matches;

    use crate::color::HSVColor;

    use super::*;

    #[test]
    fn test_cache_obj_size() {
        let mut color_cache = ColorCache::new();
        assert_eq!(size_of_val(&color_cache), 8);

        color_cache.cache_color(4, &HSVColor::new(100, 0, 100));
        assert_eq!(size_of_val(&color_cache), 8);
        assert_eq!(
            size_of_val(&color_cache.single_color_cache.unwrap().deref()),
            8
        )
    }

    #[test]
    fn test_cache_single_color_single_led() {
        let mut color_cache = ColorCache::new();
        color_cache.cache_color(4, &HSVColor::new(100, 0, 100));
        assert_matches!(color_cache.cache_size(), 1);
        let color = color_cache.load_color(4);
        assert_matches!(color, Some(color) if color.eq(&HSVColor::new(100, 0, 100)));
        color_cache.remove_cache(4);
        assert_matches!(color_cache.cache_size(), 0)
    }

    #[test]
    fn test_cache_single_color_n_leds_ungrouped() {
        let mut color_cache = ColorCache::new();
        let color = HSVColor::new(100, 0, 100);
        color_cache.cache_color(4, &color);
        color_cache.cache_color(6, &color);
        assert_matches!(color_cache.cache_size(), 2);
        assert_matches!(color_cache.load_color(4), Some(color) if color.eq(&color));
        assert_matches!(color_cache.load_color(6), Some(color) if color.eq(&color));
        color_cache.remove_cache(4);
        assert_matches!(color_cache.cache_size(), 1);
        color_cache.remove_cache(6);
        assert_matches!(color_cache.cache_size(), 0);
    }

    #[test]
    fn test_cache_single_color_n_leds_grouped() {
        let mut color_cache = ColorCache::new();
        let color = HSVColor::new(100, 0, 100);
        color_cache.cache_color(4, &color);
        color_cache.cache_color(5, &color);
        assert_matches!(color_cache.cache_size(), 2);
        assert_matches!(color_cache.load_color(4), Some(color) if color.eq(&color));
        assert_matches!(color_cache.load_color(5), Some(color) if color.eq(&color));
        color_cache.remove_cache(4);
        assert_matches!(color_cache.cache_size(), 1);
        color_cache.remove_cache(5);
        assert_matches!(color_cache.cache_size(), 0);
    }
}
