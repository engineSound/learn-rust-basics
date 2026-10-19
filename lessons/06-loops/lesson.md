---
id: rust-06-loops
track: rust
title: loop, while, for and ranges
topic: Loops
minutes: 7
concepts: [loop, while, for, ranges, break, continue]
prereqs: [rust-05-if-else]
contrast: java
edit: src/lib.rs
---

# loop, while, for and ranges

## Learn the idea

Rust has three loops, and no C-style `for (int i = 0; i < n; i++)`:

| Rust | Java | Use it for |
|------|------|------|
| `for x in items { … }` | `for (var x : items)` | going through anything iterable, including ranges |
| `while cond { … }` | `while (cond)` | repeating while something holds |
| `loop { … }` | `while (true)` | repeating until you `break`; a `loop` can even `break` *with a value* |

**Ranges:** `0..3` is 0, 1, 2 (end excluded). `1..=3` is 1, 2, 3 (end included). `break` leaves the loop and `continue` skips to the next round, as in Java.

```anatomy
{
 "title": "Anatomy of a for loop",
 "code": "for i in 1..=3 {\n    println!(\"{i}\");\n}",
 "parts": [
  {
   "text": "for",
   "kind": "keyword",
   "say": "Repeats the body once per item.",
   "java": "The enhanced `for (int i : …)`."
  },
  {
   "text": "i",
   "kind": "name",
   "say": "A new binding for each round. No `let`, no type: it is inferred from the range."
  },
  {
   "text": "in",
   "kind": "keyword",
   "say": "Separates the name from what it walks through.",
   "java": "The `:` in Java's for-each."
  },
  {
   "text": "1",
   "kind": "literal",
   "say": "Where the range starts."
  },
  {
   "text": "..=",
   "kind": "operator",
   "say": "An **inclusive** range: the end is part of it. Plain `..` would stop before the end.",
   "java": "No range syntax; you'd write `i <= 3` by hand."
  },
  {
   "text": "3",
   "kind": "literal",
   "say": "Where the range ends."
  },
  {
   "text": "{",
   "kind": "symbol",
   "say": "The body, run three times: with 1, 2 and 3."
  }
 ]
}
```


```run
for i in 0..3 {
    print!("{i} ");
}
println!();

let mut n = 1;
while n < 100 {
    n *= 3;
}
println!("{n}");

let mut tries = 0;
let found = loop {
    tries += 1;
    if tries == 4 {
        break tries * 10; // the loop's value
    }
};
println!("{found}");
```

## Quick check

```predict
for i in 1..4 {
    print!("{i}");
}
---
options: 123 | 1234 | 0123
answer: 123
why: `1..4` stops before 4.
```

```predict
let mut s = 0;
for i in 1..=5 {
    if i % 2 == 0 {
        continue;
    }
    s += i;
}
println!("{s}");
---
options: 9 | 15 | 6
answer: 9
why: `continue` skips the even numbers: 1 + 3 + 5 = 9.
```

## Your turn

- `sum_to(n)`: add 1 + 2 + … + n (0 for n = 0). Use a `for` loop over a range.
- `count_over(readings, limit)`: how many readings are **above** the limit. `readings: &[i32]` is a list you can loop over with `for r in readings`; inside the loop, `r` is a reference, so compare with `*r > limit`.

## Example

```
sum_to(4)                         →  10
count_over(&[18, 25, 31, 22], 21)   →  3
```

## Hints

1. `let mut total = 0;` before the loop, then `for i in 1..=n { total += i; }`.
2. For counting: `let mut count = 0;` and `if *r > limit { count += 1; }` inside `for r in readings`.
3. Return the counter as the last line, without `;`.

## Review

Tomorrow, write `sum_to` with `while`, then with `loop` and `break`.
