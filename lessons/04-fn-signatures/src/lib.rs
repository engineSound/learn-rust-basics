// Write two functions here, with `pub fn`:
//   area(width, height)  — u32 × u32 → u32
//   to_seconds(minutes)  — u32 → u32 (minutes × 60)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_multiplies_width_and_height() {
        assert_eq!(area(3, 4), 12);
    }

    #[test]
    fn area_of_a_line_is_zero() {
        assert_eq!(area(5, 0), 0);
    }

    #[test]
    fn two_minutes_are_120_seconds() {
        assert_eq!(to_seconds(2), 120);
    }
}
