---
id: rust-05-if-else
track: rust
title: if, else and comparisons
topic: Conditions
minutes: 6
concepts: [if, else, operators, bool]
prereqs: [rust-04-fn-signatures]
contrast: java
edit: src/lib.rs
---

# if, else and comparisons

## Learn the idea

`if` works as in Java with two differences: **no parentheses** around the condition (braces are always required), and the condition must be a real `bool`. There is no "truthy" `0` or empty string.

`if` is also an **expression**: it can produce a value, which replaces Java's `? :`.

| Operator | Meaning | Same in Java? |
|---|---|---|
| `==` `!=` | equal, not equal (compares values, also for strings) | for strings Java needs `.equals` |
| `<` `>` `<=` `>=` | comparisons | yes |
| `&&` `\|\|` `!` | and, or, not | yes |

```anatomy
{
 "title": "Anatomy of an if expression",
 "code": "let state = if level < 20 { \"low\" } else { \"ok\" };",
 "parts": [
  {
   "text": "let",
   "kind": "keyword",
   "say": "The binding gets whatever the `if` produces."
  },
  {
   "text": "state",
   "kind": "name",
   "say": "Its type is `&str`: both branches produce text."
  },
  {
   "text": "if",
   "kind": "keyword",
   "say": "Starts a condition. No parentheses needed.",
   "java": "`level < 20 ? \"low\" : \"ok\"`"
  },
  {
   "text": "level < 20",
   "kind": "operator",
   "say": "The condition: must be a `bool`. `if level { … }` would not compile."
  },
  {
   "text": "{ \"low\" }",
   "kind": "literal",
   "say": "The value when the condition is true: an expression without `;`."
  },
  {
   "text": "else",
   "kind": "keyword",
   "say": "Required when the `if` produces a value: every path must give one."
  },
  {
   "text": "{ \"ok\" }",
   "kind": "literal",
   "say": "The value otherwise. Both branches must have the same type."
  },
  {
   "text": ";",
   "kind": "symbol",
   "say": "Ends the whole `let` statement."
  }
 ]
}
```


```run
let temp = 25;
let feel = if temp > 30 { "hot" } else if temp > 15 { "mild" } else { "cold" };
println!("{temp}° is {feel}");
let door_open = false;
if !door_open && temp < 28 {
    println!("all good");
}
```

## Quick check

```predict
let n = 0;
if n {
    println!("yes");
}
---
options: yes | nothing | doesn't compile (E0308)
answer: doesn't compile (E0308)
why: An integer is not a `bool`; Rust has no truthiness. Write `if n != 0`.
```

```predict
let t = 25;
let s = if t > 30 { "hot" } else if t > 20 { "warm" } else { "cold" };
println!("{s}");
---
options: hot | warm | cold
answer: warm
why: 25 is not above 30, but it is above 20.
```

## Your turn

- `status(level, charging)`: return `"charging"` when charging; otherwise `"low"` under 20, else `"ok"`.
- `is_valid(level)`: an `i32` is a valid battery level when it is from 0 to 100 inclusive.

## Example

```
status(10, true)   →  "charging"
status(10, false)  →  "low"
status(80, false)  →  "ok"
is_valid(101)      →  false
```

## Hints

1. Check `charging` first: `if charging { … }`.
2. Use `else if level < 20` and `else`. Each branch gives a `String`: `String::from("low")`.
3. For `is_valid`: `level >= 0 && level <= 100`.

## Review

Tomorrow, rewrite `status` with the `if` as a `let` and compare.
