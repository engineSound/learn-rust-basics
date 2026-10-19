---
id: rust-09-mod-use-pub
track: rust
title: mod, use, pub and ::
topic: Modules & paths
minutes: 7
concepts: [mod, use, pub, paths]
prereqs: [rust-08-enum-match]
contrast: java
edit: src/lib.rs
---

# mod, use, pub and ::

## Learn the idea

Code is organised in **modules**. Everything is **private to its module unless marked `pub`**. Java's default is package-private; Rust's default is stricter.

| Java | Rust |
|------|------|
| `package hub.sensors;` | `mod sensors { … }` (or a file `sensors.rs`) |
| `import java.util.HashMap;` | `use std::collections::HashMap;` |
| `public` | `pub` |
| `hub.sensors.Reader` | `crate::sensors::Reader` |

In a path, `::` separates the parts. Paths start from `crate` (this project's root), `super` (the parent module), `self` (this module), or a library name like `std`.

```anatomy
{
 "title": "Anatomy of a use line",
 "code": "use std::collections::HashMap;",
 "parts": [
  {
   "text": "use",
   "kind": "keyword",
   "say": "Brings a name into scope, so you can write `HashMap` instead of the full path. It copies nothing and costs nothing at runtime.",
   "java": "`import`"
  },
  {
   "text": "std",
   "kind": "name",
   "say": "The standard library, available in every program.",
   "java": "`java.*`"
  },
  {
   "text": "::",
   "kind": "symbol",
   "say": "Path separator, between a module and what's inside it.",
   "java": "`.` in `java.util`"
  },
  {
   "text": "collections",
   "kind": "name",
   "say": "A module inside `std`.",
   "java": "`util`"
  },
  {
   "text": "HashMap",
   "kind": "type",
   "say": "The item being imported: here a type."
  },
  {
   "text": ";",
   "kind": "symbol",
   "say": "Ends the line."
  }
 ]
}
```


```run
mod units {
    pub fn to_fahrenheit(c: f64) -> f64 {
        c * 9.0 / 5.0 + 32.0
    }

    pub mod rounding {
        pub fn one_decimal(x: f64) -> f64 {
            (x * 10.0).round() / 10.0
        }
    }
}

use units::rounding::one_decimal;

fn main() {
    let f = units::to_fahrenheit(21.3);
    println!("{}", one_decimal(f));
}
```

## Quick check

```predict
mod sensors {
    fn read() -> i32 {
        21
    }
}

fn main() {
    println!("{}", sensors::read());
}
---
options: 21 | 0 | doesn't compile (E0603)
answer: doesn't compile (E0603)
why: `read` is private to `sensors`. E0603: function `read` is private. Add `pub`.
```

```predict
mod a {
    pub mod b {
        pub fn two() -> i32 {
            2
        }
    }
}

use a::b;

fn main() {
    println!("{}", b::two() + 1);
}
---
options: 2 | 3 | doesn't compile
answer: 3
why: `use a::b;` brings the module `b` into scope, so `b::two()` works.
```

## Your turn

This file doesn't compile yet; fix it in two steps, reading each error:

1. **E0425** (cannot find function `average`): add a `use` line so `hub_average` can call `average` by its short name.
2. **E0603** (function `average` is private): make `average` usable outside `sensors`.

Then finish `sensors::max_of`, and make it usable from outside too.

## Example

```
hub_average(20.0, 22.0)          →  21.0
sensors::max_of(20.0, 22.5)      →  22.5
```

## Hints

1. The full path is `sensors::average`, so the `use` line is `use sensors::average;` (or `use crate::sensors::average;`).
2. Put `pub` in front of `fn average`, and of `fn max_of`.
3. `max_of` can be `if a > b { a } else { b }`, or `a.max(b)`.

## Review

Tomorrow, move `sensors` into its own file in your head: `mod sensors;` in `lib.rs` plus `sensors.rs`. Which line changes?
