use std::process::Command;

fn run() -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_lesson-01-fn-main")).output().expect("the program runs");
    assert!(out.status.success(), "the program crashed");
    String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect()
}

#[test]
fn the_program_prints_two_lines() {
    assert_eq!(run().len(), 2);
}

#[test]
fn the_first_line_is_hub_starting() {
    assert_eq!(run().first().map(String::as_str), Some("Hub starting"));
}

#[test]
fn the_second_line_counts_devices() {
    assert_eq!(run().get(1).map(String::as_str), Some("3 devices online"));
}
