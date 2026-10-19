---
id: rust-07-struct-impl
track: rust
title: struct, impl, self and Self
topic: Structs & methods
minutes: 8
concepts: [struct, impl, self, Self, pub]
prereqs: [rust-06-loops]
contrast: java
edit: src/lib.rs
---

# struct, impl, self and Self

## Learn the idea

Rust has no classes. **Data** goes in a `struct`, **behaviour** goes in a separate `impl` block for that type.

| Java | Rust |
|------|------|
| `class Lamp { boolean on; }` | `struct Lamp { on: bool }` |
| constructor `new Lamp()` | a plain function, by convention `Lamp::new()` |
| `this` | `self` (always written out as the first parameter) |
| static method `Lamp.create()` | function without `self`: `Lamp::create()` |
| instance method `lamp.toggle()` | function with `self`: `lamp.toggle()` |

The first parameter says what a method may do: `&self` reads, `&mut self` changes, `self` takes the value over. `Self` (capital S) is just another name for the type being implemented.

```anatomy
{
 "title": "Anatomy of an impl block",
 "code": "impl Lamp {\n    fn new() -> Self {\n        Self { on: false }\n    }\n    fn toggle(&mut self) {\n        self.on = !self.on;\n    }\n}",
 "parts": [
  {
   "text": "impl",
   "kind": "keyword",
   "say": "Adds functions to a type. You can have several `impl` blocks for one type.",
   "java": "The methods inside a `class` body."
  },
  {
   "text": "Lamp",
   "kind": "type",
   "say": "The type these functions belong to (declared with `struct Lamp { on: bool }`)."
  },
  {
   "text": "fn new()",
   "kind": "name",
   "say": "Just a function named `new`: Rust has no constructors. No `self` parameter, so it is called on the type: `Lamp::new()`.",
   "java": "A constructor, or a static factory method."
  },
  {
   "text": "Self",
   "kind": "type",
   "say": "Capital S: the type being implemented, here `Lamp`.",
   "java": "Like writing the class name."
  },
  {
   "text": "Self { on: false }",
   "kind": "literal",
   "say": "Builds a value by giving every field. A struct is complete or it does not exist.",
   "java": "`new Lamp()` plus field assignments."
  },
  {
   "text": "&mut self",
   "kind": "keyword",
   "say": "This method may **change** the value it is called on. Calling it needs a `let mut` lamp.",
   "java": "`this`, which in Java is always changeable."
  },
  {
   "text": "self.on",
   "kind": "name",
   "say": "Field access through `self`. `self` is never implicit.",
   "java": "`this.on`, or just `on`."
  },
  {
   "text": "!",
   "kind": "operator",
   "say": "Logical not."
  }
 ]
}
```


```run
struct Lamp {
    on: bool,
}

impl Lamp {
    fn new() -> Self {
        Self { on: false }
    }
    fn toggle(&mut self) {
        self.on = !self.on;
    }
    fn label(&self) -> &str {
        if self.on { "on" } else { "off" }
    }
}

fn main() {
    let mut lamp = Lamp::new();
    lamp.toggle();
    println!("the lamp is {}", lamp.label());
}
```

## Quick check

```predict
struct Lamp {
    on: bool,
}

impl Lamp {
    fn toggle(&mut self) {
        self.on = !self.on;
    }
}

fn main() {
    let lamp = Lamp { on: false };
    lamp.toggle();
    println!("{}", lamp.on);
}
---
options: true | false | doesn't compile (E0596)
answer: doesn't compile (E0596)
why: `toggle` takes `&mut self`, but `lamp` was not declared `mut`. E0596: cannot borrow as mutable.
```

```predict
struct P {
    x: i32,
}

impl P {
    fn new() -> Self {
        Self { x: 7 }
    }
    fn twice(&self) -> i32 {
        self.x * 2
    }
}

fn main() {
    println!("{}", P::new().twice());
}
---
options: 7 | 14 | doesn't compile
answer: 14
why: `P::new()` builds the value (called on the type), `.twice()` reads it (called on the value).
```

## Your turn

The `Thermostat` struct is given. Write its `impl` block with three functions, all `pub`:

- `new(target)` returns a `Thermostat` with that target.
- `raise(&mut self, by)` adds `by` to the target.
- `needs_heat(&self, reading)` is `true` when the reading is below the target.

Until they exist, the checks don't compile (E0599: no function named `new`).

## Example

```
let mut t = Thermostat::new(20.0);
t.raise(1.5);           // target is now 21.5
t.needs_heat(19.0)      →  true
```

## Hints

1. Start with `impl Thermostat { }` and put `pub fn new(target: f64) -> Self { … }` inside.
2. `Self { target }` builds it (field and variable have the same name, so one word is enough). `raise` does `self.target += by;`.
3. `pub fn needs_heat(&self, reading: f64) -> bool { reading < self.target }`.

## Review

Tomorrow, add a `lower(&mut self, by)` method, then try calling it on a lamp declared without `mut`.
