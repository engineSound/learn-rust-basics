# Rust Basics

The bedrock of Rust, one keyword at a time: what `fn` means, where a
program starts, what each symbol in a line does. Same app as
[`learn-rust`](https://github.com/engineSound/learn-rust), with lessons that take code apart
token by token (tap a token to see what it means and its Java equivalent),
then a quick check, then a tiny exercise checked by real tests.

## Run it

```
cargo run                # then open http://localhost:3003
```

It runs next to `learn-rust` (different port), so you can keep both open.
Optional AI tutor: put `ANTHROPIC_API_KEY` in `.env` (see `.env.example`), as in
`learn-rust`.

## Lessons

| # | Lesson | Keywords and symbols |
|---|--------|----------------------|
| 01 | fn main: where a program starts | `fn`, `main`, `println!`, the `!` of macros |
| 02 | let and mut: naming values | `let`, `mut`, shadowing, `const` |
| 03 | Types, literals and as | `i32`/`u8`/`f64`/`bool`/`char`, `as` |
| 04 | fn signatures and return values | parameters, `->`, the last expression, `return` |
| 05 | if, else and comparisons | `if`/`else` as an expression, `==` `&&` `||` `!` |
| 06 | loop, while, for and ranges | `loop`, `while`, `for … in`, `..` / `..=`, `break`, `continue` |
| 07 | struct, impl, self and Self | `struct`, `impl`, `&self`, `&mut self`, `Self`, `::new` |
| 08 | enum, match and Option | `enum`, `match`, `=>`, `_`, `Some`/`None` |
| 09 | mod, use, pub and :: | modules, privacy, paths: `crate`, `super`, `std` |
| 10 | & and &mut: borrowing | `&`, `&mut`, `*`, moves |

Every keyword and reserved word, with the lesson that covers it: [docs/KEYWORDS.md](docs/KEYWORDS.md).

## Docs

- [CLAUDE.md](CLAUDE.md): how lessons are written here
- [docs/SHARED-FORMAT.md](docs/SHARED-FORMAT.md): lesson and history formats (shared by all four learn-* repos)
- [PROGRESS.md](PROGRESS.md): your learning log and generated profile
