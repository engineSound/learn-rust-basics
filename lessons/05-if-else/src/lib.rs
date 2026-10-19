/// "charging" when charging; otherwise "low" under 20, else "ok".
pub fn status(level: u8, charging: bool) -> String {
    todo!()
}

/// A level is valid from 0 to 100 inclusive.
pub fn is_valid(level: i32) -> bool {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charging_wins_over_everything() {
        assert_eq!(status(5, true), "charging");
    }

    #[test]
    fn under_twenty_is_low() {
        assert_eq!(status(19, false), "low");
    }

    #[test]
    fn twenty_and_up_is_ok() {
        assert_eq!(status(20, false), "ok");
    }

    #[test]
    fn zero_to_one_hundred_are_valid() {
        assert!(is_valid(0) && is_valid(100));
        assert!(!is_valid(-1) && !is_valid(101));
    }
}
