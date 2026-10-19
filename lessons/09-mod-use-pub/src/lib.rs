pub mod sensors {
    fn average(a: f64, b: f64) -> f64 {
        (a + b) / 2.0
    }

    fn max_of(a: f64, b: f64) -> f64 {
        todo!()
    }
}

// 1. Add a `use` line here.

pub fn hub_average(a: f64, b: f64) -> f64 {
    average(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hub_averages_two_readings() {
        assert_eq!(hub_average(20.0, 22.0), 21.0);
    }

    #[test]
    fn average_is_usable_from_outside_the_module() {
        assert_eq!(sensors::average(1.0, 2.0), 1.5);
    }

    #[test]
    fn max_of_picks_the_larger_reading() {
        assert_eq!(sensors::max_of(20.0, 22.5), 22.5);
        assert_eq!(sensors::max_of(-1.0, -3.0), -1.0);
    }
}
