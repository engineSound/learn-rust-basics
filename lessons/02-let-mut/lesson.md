---
id: rust-02-let-mut
track: rust
title: let and mut: naming values
topic: Bindings
minutes: 6
concepts: [let, mut, shadowing, const]
prereqs: [rust-01-fn-main]
contrast: java
edit: src/lib.rs
---

# let and mut: naming values

## Learn the idea

`let` gives a value a name. In Rust a name is **immutable unless you say `mut`**, the opposite of Java, where you'd add `final` to forbid changes.

| Java | Rust |
|------|------|
| `final int max = 100;` | `let max = 100;` |
| `int count = 0; count++;` | `let mut count = 0; count += 1;` |
| `static final int LIMIT = 5;` | `const LIMIT: u32 = 5;` |

The type usually comes from the value (`let x = 5;` is an `i32`). When you write it, it goes **after** the name, with a colon.

```anatomy
{
 "title": "Anatomy of a binding",
 "code": "let mut count: u32 = 0;",
 "parts": [
  {
   "text": "let",
   "kind": "keyword",
   "say": "Introduces a new variable (Rust calls it a **binding**: a name bound to a value).",
   "java": "`int count = 0;`: Java starts with the type; Rust starts with `let`."
  },
  {
   "text": "mut",
   "kind": "keyword",
   "say": "Allows the value to change later. Without it, `count = 1;` is a compile error (E0384).",
   "java": "Java variables are changeable unless `final`; Rust is the reverse."
  },
  {
   "text": "count",
   "kind": "name",
   "say": "The name. Rust style is `snake_case`.",
   "java": "Java style would be `camelCase`."
  },
  {
   "text": ":",
   "kind": "symbol",
   "say": "Says \"a type annotation follows\". Optional when Rust can infer the type.",
   "java": "Java puts the type before the name instead."
  },
  {
   "text": "u32",
   "kind": "type",
   "say": "An unsigned 32-bit integer: whole numbers from 0 up to about 4 billion.",
   "java": "Java has no unsigned `int`; the closest is `int`."
  },
  {
   "text": "=",
   "kind": "operator",
   "say": "Gives the binding its first value.",
   "java": "Same as Java."
  },
  {
   "text": "0",
   "kind": "literal",
   "say": "An integer literal.",
   "java": "Same as Java."
  },
  {
   "text": ";",
   "kind": "symbol",
   "say": "Ends the statement.",
   "java": "Same as Java."
  }
 ]
}
```

**Shadowing:** writing `let` again with the same name makes a *new* binding that hides the old one. It can even have a new type. This is not mutation, and it's common in Rust.

```run
let mut count = 0;
count += 1;
count += 1;
let label = "devices";
println!("{count} {label}");
let count = count * 10; // shadowing: a new `count`
println!("{count}");
```

## Quick check

```predict
let x = 5;
let x = x + 1;
println!("{x}");
---
options: 5 | 6 | doesn't compile (E0384)
answer: 6
why: The second `let` makes a new `x` (shadowing), so nothing is mutated.
```

```predict
let x = 5;
x = 6;
println!("{x}");
---
options: 5 | 6 | doesn't compile (E0384)
answer: doesn't compile (E0384)
why: Assigning to `x` needs `let mut x`. E0384: cannot assign twice to immutable variable.
```

## Your turn

Two small functions. The first one **doesn't compile** yet: run it, read the E0384 error, then fix it with `mut`.

- `total_minutes(hours, minutes)`: hours × 60 plus minutes.
- `countdown_label(seconds)`: write it yourself. Use a `let` binding named `text` for `"seconds left"`, then return `format!("{seconds} {text}")`.

## Example

```
total_minutes(2, 15)     →  135
countdown_label(30)      →  "30 seconds left"
```

## Hints

1. E0384 points at the line that assigns to `total` a second time. Which binding needs to be changeable?
2. Write `let mut total = hours * 60;`.
3. For the label: `let text = "seconds left";` then `format!("{seconds} {text}")` as the last line, without `;`.

## Review

Tomorrow, explain out loud the difference between `let mut x` and writing `let x` twice.
