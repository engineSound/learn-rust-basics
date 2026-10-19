---
id: rust-03-types-as
track: rust
title: Types, literals and as
topic: Primitive types
minutes: 6
concepts: [types, literals, as]
prereqs: [rust-02-let-mut]
contrast: java
edit: src/lib.rs
---

# Types, literals and as

## Learn the idea

Rust's basic types are spelled by size, and **numbers never convert silently**: you convert with `as`.

| Rust | Java | Note |
|------|------|------|
| `i8 i16 i32 i64` | `byte short int long` | signed; `i32` is the default |
| `u8 u16 u32 u64` | none | unsigned: 0 and up |
| `usize` | `int` (for indexes) | the size of memory addresses; used for lengths and indexes |
| `f32 f64` | `float double` | `f64` is the default |
| `bool` | `boolean` | `true` / `false` |
| `char` | `char` | 4 bytes: any Unicode character, in **single** quotes |
| `&str`, `String` | `String` | borrowed text vs owned text |

`2` is an integer and `2.0` is a float, and they don't mix: `2 * 1.5` doesn't compile.

```anatomy
{
 "title": "Anatomy of a conversion",
 "code": "let percent = level as f64 / 255.0 * 100.0;",
 "parts": [
  {
   "text": "let",
   "kind": "keyword",
   "say": "A new binding (lesson 02)."
  },
  {
   "text": "percent",
   "kind": "name",
   "say": "Its type is inferred: `f64`, because the right side is an `f64`."
  },
  {
   "text": "level",
   "kind": "name",
   "say": "Suppose `level` is a `u8` (0–255) from a sensor."
  },
  {
   "text": "as",
   "kind": "keyword",
   "say": "Converts a number to another number type, explicitly. Rust never widens or narrows behind your back.",
   "java": "Java widens `int` to `double` automatically; narrowing needs a cast `(int) x`."
  },
  {
   "text": "f64",
   "kind": "type",
   "say": "The target type: a 64-bit float.",
   "java": "`double`"
  },
  {
   "text": "/",
   "kind": "operator",
   "say": "Division. Both sides are `f64` now, so this is float division."
  },
  {
   "text": "255.0",
   "kind": "literal",
   "say": "A float literal. Writing `255` here would be an integer, and `f64 / i32` doesn't compile."
  },
  {
   "text": "*",
   "kind": "operator",
   "say": "Multiplication."
  },
  {
   "text": "100.0",
   "kind": "literal",
   "say": "Another float literal."
  }
 ]
}
```


```run
let small: u8 = 200;
let big: i64 = small as i64 * 1000;
let ratio = small as f64 / 255.0;
let letter: char = 'é';
println!("{big} {ratio:.2} {letter} {}", letter.len_utf8());
```

## Quick check

```predict
println!("{}", 7 / 2);
---
options: 3 | 3.5 | 4
answer: 3
why: Two integers divide to an integer: the fraction is dropped, as in Java.
```

```predict
let x: i32 = 300;
println!("{}", x as u8);
---
options: 300 | 255 | 44
answer: 44
why: `as` between integers keeps the low bits: 300 − 256 = 44. It never fails, so check ranges yourself.
```

## Your turn

- `to_percent(raw)`: a sensor reports a `u8` from 0 to 255. Return the percentage as a `u8`, **rounded** (128 → 50).
- `average(a, b)`: two `u32` values, return their average as an `f64` (3 and 4 → 3.5).

## Example

```
to_percent(255)  →  100
to_percent(128)  →  50
average(3, 4)    →  3.5
```

## Hints

1. Convert first: `raw as f64`. Then divide by `255.0` and multiply by `100.0`.
2. Floats have `.round()`. Convert back with `as u8` at the end.
3. `(raw as f64 / 255.0 * 100.0).round() as u8` and `(a as f64 + b as f64) / 2.0`.

## Review

Tomorrow, predict `7.0 / 2.0`, `-7 / 2` and `250u8 as i8`, then check them in a run block.
