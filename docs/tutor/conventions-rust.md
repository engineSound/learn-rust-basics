## File conventions (Rust track)

- `starter`: the content of `src/lib.rs` *without* the test module. Each `pub fn` has a `///` doc comment with an example and a body of exactly `todo!()`. It must compile as is (unused-variable warnings are fine).
- `tests`: the test module only, starting with `#[cfg(test)]` and `mod tests {`, containing `use super::*;` and `#[test]` functions with descriptive snake_case names. The app appends it to `starter` and to `solution`.
- `solution`: a correct `src/lib.rs` *without* the test module, with the same signatures as `starter`.
- Rust 2021 edition, standard library only, no `unsafe`, no external crates.
- In `explanation`, ```run and ```predict blocks are the body of `fn main` (the app wraps them): print with `println!`.
- Write assertions as `assert_eq!(actual_call(...), expected)` so failures show the expected and actual values.
