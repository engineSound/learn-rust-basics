/// 2 hours 15 minutes → 135 minutes.
pub fn total_minutes(hours: u32, minutes: u32) -> u32 {
    let total = hours * 60;
    total = total + minutes; // E0384: fix this with `mut`
    total
}

/// 30 → "30 seconds left"
pub fn countdown_label(seconds: u32) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_hours_fifteen_is_135_minutes() {
        assert_eq!(total_minutes(2, 15), 135);
    }

    #[test]
    fn zero_hours_is_just_the_minutes() {
        assert_eq!(total_minutes(0, 7), 7);
    }

    #[test]
    fn the_label_puts_the_number_first() {
        assert_eq!(countdown_label(30), "30 seconds left");
    }
}
