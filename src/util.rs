pub fn range_map<T>(input: T, input_start: T, input_end: T, output_start: T, output_end: T) -> T
where
    T: core::ops::Add<Output = T>
        + core::ops::Div<Output = T>
        + core::ops::Mul<Output = T>
        + core::ops::Sub<Output = T>
        + Copy,
{
    output_start + ((output_end - output_start) / (input_end - input_start)) * (input - input_start)
}

pub fn range_map_to_0_1(input: f32, input_start: f32, input_end: f32) -> f32 {
    range_map(input, input_start, input_end, 0f32, 1f32)
}

pub fn range_map_from_0_1(input: f32, output_start: f32, output_end: f32) -> f32 {
    range_map(input, 0f32, 1f32, output_start, output_end)
}

pub fn max_3(a: f32, b: f32, c: f32) -> f32 {
    if a > b && a > c {
        a
    } else if c > a && c > b {
        c
    } else {
        b
    }
}

pub fn min_3(a: f32, b: f32, c: f32) -> f32 {
    if a <= b && a <= c {
        a
    } else if c <= a && c <= b {
        c
    } else {
        b
    }
}

pub fn wrap_on<T: num_traits::Unsigned + Ord>(value: T, max: T) -> T {
    if value > max {
        value - max
    } else {
        value
    }
}
