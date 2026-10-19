# Rust keywords

Every Rust keyword, what it does, its Java equivalent, and the lesson that
teaches it. "Later" means `learn-rust` covers it (or will).

## Used every day

| Keyword | What it does | Java | Lesson |
|---|---|---|---|
| `fn` | defines a function | (return type before the name) | 01, 04 |
| `let` | declares a binding (a variable) | `int x =` | 02 |
| `mut` | makes a binding or reference changeable | (default; `final` is the opposite) | 02, 10 |
| `const` | a compile-time constant | `static final` | 02 |
| `as` | converts between number types; renames in `use … as …` | `(int) x` | 03 |
| `true` `false` | the two `bool` values | same | 05 |
| `if` `else` | conditions; also an expression | `if`, `? :` | 05 |
| `loop` | repeats until `break`; can return a value | `while (true)` | 06 |
| `while` | repeats while a condition holds | same | 06 |
| `for` `in` | goes through an iterator or range | `for (x : xs)` | 06 |
| `break` `continue` | leave the loop / skip to the next round | same | 06 |
| `return` | leaves a function early | same | 04 |
| `struct` | a type with named fields | `class` / `record` (data only) | 07 |
| `impl` | adds functions to a type; implements a trait | methods in a class; `implements` | 07 |
| `self` | the value a method is called on; also "this module" in paths | `this` | 07, 09 |
| `Self` | the type being implemented | the class name | 07 |
| `pub` | makes an item visible outside its module | `public` | 07, 09 |
| `enum` | a type with a fixed set of variants, which can carry data | `enum`, sealed interface | 08 |
| `match` | picks a branch by pattern; must cover every case | `switch` expression | 08 |
| `mod` | declares a module | `package` | 09 |
| `use` | brings a path into scope | `import` | 09 |
| `crate` | the root of the current project, in paths | (the top package) | 09 |
| `super` | the parent module, in paths | (none) | 09 |

## Later (in `learn-rust`)

| Keyword | What it does | Java |
|---|---|---|
| `trait` | a set of methods types can implement | `interface` |
| `where` | spells out generic bounds | `<T extends …>` |
| `dyn` | a trait object: "some type implementing this trait" | an interface-typed variable |
| `type` | a type alias | (none) |
| `static` | a global with a fixed address | `static` field |
| `ref` | binds by reference inside a pattern | (none) |
| `move` | a closure takes ownership of what it captures | (none) |
| `async` `await` | asynchronous functions and waiting on futures | `CompletableFuture` |
| `unsafe` | code the compiler can't check (raw pointers, FFI) | (JNI, `sun.misc.Unsafe`) |
| `extern` | links to code in other languages | `native` |

## Reserved for the future (not usable as names)

`abstract`, `become`, `box`, `do`, `final`, `gen`, `macro`, `override`,
`priv`, `try`, `typeof`, `unsized`, `virtual`, `yield`.

## Not keywords, but look like them

- `main`: an ordinary name; a binary's entry point by convention (lesson 01).
- `println!`, `format!`, `vec!`, `todo!`: macros; the `!` marks a macro call (lesson 01).
- `Some`, `None`, `Ok`, `Err`: variants of the standard `Option` and `Result` enums (lesson 08).
- `String`, `Vec`: ordinary standard-library types.
- `'static`, `'a`: lifetimes (later).
