/// A raw sensor byte (0..=255) as a rounded percentage: 128 → 50.
pub fn to_percent(raw: u8) -> u8 {
    todo!()
}

/// The average of two whole numbers, as a float: (3, 4) → 3.5.
pub fn average(a: u32, b: u32) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_zero_percent() {
        assert_eq!(to_percent(0), 0);
    }

    #[test]
    fn full_scale_is_one_hundred_percent() {
        assert_eq!(to_percent(255), 100);
    }

    #[test]
    fn halfway_rounds_to_fifty() {
        assert_eq!(to_percent(128), 50);
    }

    #[test]
    fn the_average_keeps_the_half() {
        assert_eq!(average(3, 4), 3.5);
    }
}
