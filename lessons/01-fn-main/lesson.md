---
id: rust-01-fn-main
track: rust
title: fn main: where a program starts
topic: Entry point
minutes: 5
concepts: [fn, main, println, macros]
prereqs: []
contrast: java
edit: src/main.rs
---

# fn main: where a program starts

## Learn the idea

Every Rust program that you *run* starts at one function: **`main`**. When you type `cargo run`, Cargo compiles `src/main.rs` and calls `main()`. When `main` ends, the program ends.

| Java | Rust |
|------|------|
| `public class App { public static void main(String[] args) { … } }` | `fn main() { … }` |
| needs a class | no class: functions stand alone |
| `System.out.println("Hi");` | `println!("Hi");` |

A project with `src/main.rs` and `fn main` is a **binary** (something you run). A project with `src/lib.rs` is a **library**: no `main`, other code calls its functions. Click through each piece below.

```anatomy
{
 "title": "Anatomy of the smallest Rust program",
 "code": "fn main() {\n    println!(\"Hello, device!\");\n}",
 "parts": [
  {
   "text": "fn",
   "kind": "keyword",
   "say": "Starts a function definition. Short for *function*. Every function in Rust begins with `fn`.",
   "java": "No keyword: Java recognises a method by its return type, `void main(…)`."
  },
  {
   "text": "main",
   "kind": "name",
   "say": "The **entry point**. When the program starts, Rust calls `main` first. A binary has exactly one.",
   "java": "`public static void main`, without `public`, `static`, `void` or a class around it."
  },
  {
   "text": "()",
   "kind": "symbol",
   "say": "The parameter list, empty: `main` takes no arguments. Command-line arguments come from `std::env::args()` instead.",
   "java": "`(String[] args)`"
  },
  {
   "text": "{",
   "kind": "symbol",
   "say": "Opens the function body.",
   "java": "Same as Java."
  },
  {
   "text": "println!",
   "kind": "macro",
   "say": "Prints a line. The `!` means it is a **macro**, not a function: it is expanded at compile time, which is how it can check the format string and take any number of values.",
   "java": "`System.out.println`"
  },
  {
   "text": "\"Hello, device!\"",
   "kind": "literal",
   "say": "A string literal. Its type is `&str`: text baked into the program.",
   "java": "A `String` literal."
  },
  {
   "text": ";",
   "kind": "symbol",
   "say": "Ends the statement.",
   "java": "Same as Java."
  },
  {
   "text": "}",
   "kind": "symbol",
   "say": "Closes the body. When `main` returns, the program exits (code 0).",
   "java": "Same as Java."
  }
 ]
}
```

`{}` in the format string is a placeholder, filled by the values after it, in order:

```run
fn main() {
    println!("Hello, device!");
    println!("{} + {} = {}", 2, 3, 2 + 3);
}
```

## Quick check

```predict
fn main() {
    println!("A");
}

fn helper() {
    println!("B");
}
---
options: A | B | A then B
answer: A
why: Only `main` runs by itself. `helper` is never called, so it never prints.
```

```predict
fn main() {
    println!("{}-{}", "door", 7);
}
---
options: door-7 | {}-{} | door7
answer: door-7
why: Each `{}` is replaced by the next value, in order.
```

## Your turn

The editor holds a program with **no `main`** yet. Run it first: the compiler says `E0601: main function not found`, because a binary can't start without one.

Then write `fn main` so the program prints exactly these two lines:

```
Hub starting
3 devices online
```

The checks run your program and compare what it prints.

## Example

```
$ cargo run
Hub starting
3 devices online
```

## Hints

1. The smallest program is `fn main() {}`. Start with that and run it.
2. One `println!` per line, each ending with `;`.
3. `println!("Hub starting");` then `println!("{} devices online", 3);` inside the braces.

## Review

Tomorrow, write a two-line hello program in an empty file without looking.
