/// Add up the values; the caller keeps its list.
pub fn total(values: &[f64]) -> f64 {
    todo!()
}

/// Set the caller's counter back to 0.
pub fn reset(counter: &mut u32) {
    todo!()
}

/// Append every value of `more` to `log`.
pub fn add_all(log: &mut Vec<f64>, more: &[f64]) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_adds_and_the_list_is_still_usable() {
        let values = vec![1.0, 2.5];
        assert_eq!(total(&values), 3.5);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn reset_changes_the_callers_counter() {
        let mut c = 7;
        reset(&mut c);
        assert_eq!(c, 0);
    }

    #[test]
    fn add_all_appends_in_order() {
        let mut log = vec![1.0];
        add_all(&mut log, &[4.0, 5.0]);
        assert_eq!(log, vec![1.0, 4.0, 5.0]);
    }
}
