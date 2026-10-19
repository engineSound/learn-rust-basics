---
id: rust-04-fn-signatures
track: rust
title: fn signatures and return values
topic: Functions
minutes: 6
concepts: [fn, parameters, return, expressions]
prereqs: [rust-03-types-as]
contrast: java
edit: src/lib.rs
---

# fn signatures and return values

## Learn the idea

A function's **signature** says what goes in and what comes out. Every parameter needs a type; the return type comes after `->`.

The big difference from Java: **the last expression of a body is its return value**, written *without* `;`. `return` exists too, mainly to leave early.

| Java | Rust |
|------|------|
| `int add(int a, int b) { return a + b; }` | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `void log(String s)` | `fn log(s: &str)` (no `->` means "returns nothing") |

```anatomy
{
 "title": "Anatomy of a signature",
 "code": "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
 "parts": [
  {
   "text": "fn",
   "kind": "keyword",
   "say": "Starts a function definition."
  },
  {
   "text": "add",
   "kind": "name",
   "say": "The function's name, in `snake_case`."
  },
  {
   "text": "a",
   "kind": "name",
   "say": "First parameter."
  },
  {
   "text": ": i32",
   "kind": "type",
   "say": "Each parameter has its type after a colon. Unlike `let`, parameters are never inferred.",
   "java": "`int a`"
  },
  {
   "text": "b: i32",
   "kind": "name",
   "say": "Second parameter, separated by a comma."
  },
  {
   "text": "->",
   "kind": "symbol",
   "say": "\"Returns\". The type after it is what the function gives back.",
   "java": "Java writes the return type first: `int add(…)`."
  },
  {
   "text": "i32",
   "kind": "type",
   "say": "The return type."
  },
  {
   "text": "a + b",
   "kind": "operator",
   "say": "The last expression, **with no `;`**: its value is what the function returns. No `return` needed.",
   "java": "`return a + b;`"
  },
  {
   "text": "}",
   "kind": "symbol",
   "say": "End of the body."
  }
 ]
}
```

Use `return` when you want to leave before the end:

```run
fn double(n: i32) -> i32 {
    n * 2
}

fn clamp_to_100(n: i32) -> i32 {
    if n > 100 {
        return 100; // leave early
    }
    n
}

fn main() {
    println!("{} {} {}", double(21), clamp_to_100(250), clamp_to_100(42));
}
```

## Quick check

```predict
fn five() -> i32 {
    5;
}

fn main() {
    println!("{}", five());
}
---
options: 5 | 0 | doesn't compile (E0308)
answer: doesn't compile (E0308)
why: With `;`, `5;` is a statement and the body returns nothing, `()`, not an `i32`. E0308: mismatched types.
```

```predict
fn inc(x: i32) -> i32 {
    x + 1
}

fn main() {
    println!("{}", inc(inc(1)));
}
---
options: 2 | 3 | 4
answer: 3
why: Inner call first: `inc(1)` is 2, then `inc(2)` is 3.
```

## Your turn

The editor is almost empty: **write both functions yourself**, signature included. Until they exist the checks don't compile (E0425: cannot find function), which is expected.

- `area(width, height)`: two `u32`, returns their product as `u32`.
- `to_seconds(minutes)`: a `u32`, returns minutes × 60 as `u32`.

Both must be `pub` so the checks can call them: `pub fn …`.

## Example

```
area(3, 4)       →  12
to_seconds(2)    →  120
```

## Hints

1. Start with the shape: `pub fn area(width: u32, height: u32) -> u32 { }`.
2. The body is one expression with no `;`.
3. `pub fn area(width: u32, height: u32) -> u32 { width * height }`, and the same shape for `to_seconds`.

## Review

Tomorrow, write a function with two parameters from memory, then put a `;` after its last line and read E0308.
