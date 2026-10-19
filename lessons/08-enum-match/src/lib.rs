#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Light {
    Red,
    Yellow,
    Green,
}

/// Red → Green → Yellow → Red
pub fn next(light: Light) -> Light {
    todo!()
}

/// Red 30, Yellow 5, Green 25
pub fn seconds(light: Light) -> u32 {
    todo!()
}

/// The index of the first Red, or None.
pub fn first_red(lights: &[Light]) -> Option<usize> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Light::*;

    #[test]
    fn the_cycle_goes_red_green_yellow() {
        assert_eq!(next(Red), Green);
        assert_eq!(next(Green), Yellow);
        assert_eq!(next(Yellow), Red);
    }

    #[test]
    fn each_light_has_its_duration() {
        assert_eq!((seconds(Red), seconds(Yellow), seconds(Green)), (30, 5, 25));
    }

    #[test]
    fn finds_the_first_red() {
        assert_eq!(first_red(&[Green, Red, Red]), Some(1));
    }

    #[test]
    fn no_red_means_none() {
        assert_eq!(first_red(&[Green, Yellow]), None);
    }
}
