pub struct Thermostat {
    pub target: f64,
}

// Write `impl Thermostat { … }` here with:
//   pub fn new(target: f64) -> Self
//   pub fn raise(&mut self, by: f64)
//   pub fn needs_heat(&self, reading: f64) -> bool

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_the_target() {
        assert_eq!(Thermostat::new(20.0).target, 20.0);
    }

    #[test]
    fn raise_adds_to_the_target() {
        let mut t = Thermostat::new(20.0);
        t.raise(1.5);
        assert_eq!(t.target, 21.5);
    }

    #[test]
    fn a_reading_below_the_target_needs_heat() {
        let t = Thermostat::new(20.0);
        assert!(t.needs_heat(19.0));
        assert!(!t.needs_heat(20.0));
    }
}
