/// 1 + 2 + … + n
pub fn sum_to(n: u32) -> u32 {
    todo!()
}

/// How many readings are above the limit.
pub fn count_over(readings: &[i32], limit: i32) -> u32 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_to_four_is_ten() {
        assert_eq!(sum_to(4), 10);
    }

    #[test]
    fn sum_to_zero_is_zero() {
        assert_eq!(sum_to(0), 0);
    }

    #[test]
    fn counts_readings_above_the_limit() {
        assert_eq!(count_over(&[18, 25, 31, 22], 21), 3);
    }

    #[test]
    fn a_reading_equal_to_the_limit_does_not_count() {
        assert_eq!(count_over(&[21, 21], 21), 0);
    }
}
