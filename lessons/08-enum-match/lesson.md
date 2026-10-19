---
id: rust-08-enum-match
track: rust
title: enum, match and Option
topic: Enums & patterns
minutes: 8
concepts: [enum, match, option, patterns]
prereqs: [rust-07-struct-impl]
contrast: java
edit: src/lib.rs
---

# enum, match and Option

## Learn the idea

An `enum` is a type with a fixed set of **variants**. `match` picks what to do for each one, and the compiler checks that you covered them all.

| Java | Rust |
|------|------|
| `enum Mode { ECO, COMFORT }` | `enum Mode { Eco, Comfort }` |
| `Mode.ECO` | `Mode::Eco` |
| `switch (m) { case ECO -> 18; … }` | `match m { Mode::Eco => 18, … }` |
| `default ->` | `_ =>` |
| `null` | `Option`: either `Some(value)` or `None` |

`Option<T>` is just an enum from the standard library. That's how Rust has no `null`: a value that might be missing has type `Option<…>`, and you must `match` it (or use helpers like `unwrap_or`) before using it.

```anatomy
{
 "title": "Anatomy of a match",
 "code": "match mode {\n    Mode::Eco => 18,\n    Mode::Comfort => 21,\n    _ => 20,\n}",
 "parts": [
  {
   "text": "match",
   "kind": "keyword",
   "say": "Compares a value against patterns, top to bottom. It is an expression: the chosen arm gives its value.",
   "java": "`switch` expression (Java 14+)."
  },
  {
   "text": "mode",
   "kind": "name",
   "say": "The value being examined."
  },
  {
   "text": "Mode",
   "kind": "type",
   "say": "The enum's name."
  },
  {
   "text": "::",
   "kind": "symbol",
   "say": "Path separator: `Mode::Eco` means \"the `Eco` that lives in `Mode`\".",
   "java": "The `.` in `Mode.ECO`."
  },
  {
   "text": "Eco",
   "kind": "name",
   "say": "A variant, written in `CamelCase`."
  },
  {
   "text": "=>",
   "kind": "symbol",
   "say": "An **arm**: pattern on the left, result on the right.",
   "java": "The `->` in a switch case."
  },
  {
   "text": "18",
   "kind": "literal",
   "say": "The value of this arm."
  },
  {
   "text": "_",
   "kind": "symbol",
   "say": "Matches anything not matched above.",
   "java": "`default`"
  }
 ]
}
```


```run
enum Mode {
    Eco,
    Comfort,
    Away,
}

fn target(mode: Mode) -> i32 {
    match mode {
        Mode::Eco => 18,
        Mode::Comfort => 21,
        Mode::Away => 15,
    }
}

fn main() {
    println!("{} {}", target(Mode::Eco), target(Mode::Away));
    let reading: Option<i32> = None;
    println!("{}", reading.unwrap_or(-1));
    if let Some(r) = Some(42) {
        println!("got {r}");
    }
}
```

## Quick check

```predict
enum Light {
    Red,
    Green,
}

fn main() {
    let l = Light::Green;
    let s = match l {
        Light::Red => "stop",
    };
    println!("{s}");
}
---
options: stop | nothing | doesn't compile (E0004)
answer: doesn't compile (E0004)
why: `Green` is not covered. E0004: non-exhaustive patterns. Add an arm, or `_`.
```

```predict
let x: Option<i32> = None;
let n = match x {
    Some(v) => v,
    None => -1,
};
println!("{n}");
---
options: 0 | -1 | doesn't compile
answer: -1
why: `x` is `None`, so the second arm runs.
```

## Your turn

`Light` is given: `Red`, `Yellow`, `Green`. (The `#[derive(…)]` line above it lets the checks compare and print lights; you'll meet `derive` later.)

- `next(light)`: Red → Green → Yellow → Red.
- `seconds(light)`: Red 30, Yellow 5, Green 25.
- `first_red(lights)`: the position of the first `Red` in a list, as `Some(index)`, or `None` if there is none.

## Example

```
next(Light::Red)                    →  Light::Green
seconds(Light::Yellow)              →  5
first_red(&[Green, Red, Red])       →  Some(1)
first_red(&[Green])                 →  None
```

## Hints

1. `match light { Light::Red => Light::Green, … }`: one arm per variant.
2. For `first_red`, loop with an index: `for (i, l) in lights.iter().enumerate() { … }`. `*l == Light::Red` compares.
3. Inside the loop, `return Some(i);` when you find it. After the loop, `None`.

## Review

Tomorrow, add a `FlashingYellow` variant and let the compiler show you each `match` to update.
