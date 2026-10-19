---
id: rust-10-references
track: rust
title: & and &mut: borrowing
topic: References
minutes: 8
concepts: [references, borrowing, deref, ownership]
prereqs: [rust-09-mod-use-pub]
contrast: java
edit: src/lib.rs
---

# & and &mut: borrowing

## Learn the idea

In Java every object variable is secretly a reference. In Rust you choose, and the symbol shows it:

| Rust | Meaning | Java |
|------|---------|------|
| `v` | the value itself; passing it **moves** it (for `String`, `Vec`, …) | — |
| `&v` | a **shared borrow**: read it, the owner keeps it | passing an object |
| `&mut v` | an **exclusive borrow**: change it, the owner gets it back | passing an object you mutate |
| `*r` | follow the reference to the value | automatic |

The rule: at any moment, **many `&` or one `&mut`**, never both. The compiler checks it, so there are no data races and no `ConcurrentModificationException`.

```anatomy
{
 "title": "Anatomy of a borrow",
 "code": "fn add_reading(log: &mut Vec<f64>, value: f64) {\n    log.push(value);\n}\n\nadd_reading(&mut log, 21.5);",
 "parts": [
  {
   "text": "fn add_reading",
   "kind": "name",
   "say": "A function that changes the caller's list."
  },
  {
   "text": "log",
   "kind": "name",
   "say": "The parameter."
  },
  {
   "text": "&mut",
   "kind": "keyword",
   "say": "Its type is a **mutable reference**: the function may change the list, but doesn't own it. While borrowed, nobody else may use it.",
   "java": "Passing a `List<Double>` you add to."
  },
  {
   "text": "Vec<f64>",
   "kind": "type",
   "say": "A growable list of `f64`.",
   "java": "`ArrayList<Double>`"
  },
  {
   "text": "value: f64",
   "kind": "name",
   "say": "A plain `f64` is copied: small number types are `Copy`."
  },
  {
   "text": "log.push(value)",
   "kind": "name",
   "say": "Calling a method through the reference: no `*` needed with `.`."
  },
  {
   "text": "&mut log",
   "kind": "operator",
   "say": "At the call site you write `&mut` too: you can see that `log` may change. `log` must be declared `let mut`.",
   "java": "Invisible in Java."
  },
  {
   "text": "21.5",
   "kind": "literal",
   "say": "A value copied in."
  }
 ]
}
```


```run
fn largest(values: &[i32]) -> i32 {
    let mut best = values[0];
    for v in values {
        if *v > best {
            best = *v;
        }
    }
    best
}

fn bump(n: &mut i32) {
    *n += 1;
}

fn main() {
    let nums = vec![3, 9, 4];
    println!("{}", largest(&nums));
    let mut count = 0;
    bump(&mut count);
    bump(&mut count);
    println!("{count} {}", nums.len()); // nums is still ours
}
```

## Quick check

```predict
let mut a = 1;
let r = &mut a;
*r += 10;
println!("{a}");
---
options: 1 | 11 | doesn't compile
answer: 11
why: `*r` follows the reference to `a`; after the last use of `r`, `a` can be read again.
```

```predict
let s = String::from("hi");
let t = s;
println!("{s}");
---
options: hi | (nothing) | doesn't compile (E0382)
answer: doesn't compile (E0382)
why: `let t = s;` **moves** the String to `t`. E0382: borrow of moved value `s`. Write `let t = &s;` to borrow instead.
```

## Your turn

- `total(values)`: add up a list you only **read** (`&[f64]`).
- `reset(counter)`: set the caller's counter back to 0 (`&mut u32`).
- `add_all(log, more)`: append every value of `more` to the caller's `log`.

The checks also make sure the caller can still use its values afterwards.

## Example

```
total(&[1.0, 2.5])             →  3.5
let mut c = 7; reset(&mut c);  // c is 0
add_all(&mut log, &[4.0, 5.0])  // log grew by two
```

## Hints

1. `total`: `let mut sum = 0.0;` and `for v in values { sum += *v; }`.
2. `reset`: change the value behind the reference with `*counter = 0;`.
3. `add_all`: `for v in more { log.push(*v); }`.

## Review

Tomorrow, change `total` to take `Vec<f64>` instead of `&[f64]` and read the error the test gives. Why?
