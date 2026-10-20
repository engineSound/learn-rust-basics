# fn main: where a program starts

<!-- note {"anchor":"idea","at":"2026-10-20T09:00:00.000Z"} -->
## The idea

No class around `main`: closer to a Python script than to Java.

<!-- note {"anchor":"token:println!","at":"2026-10-20T09:05:00.000Z"} -->
## `println!`

The `!` means macro. Like a compile-time `String.format` + print.
Why a macro? Variadic arguments, checked at compile time.

<!-- note {"anchor":"token:fn main()","at":"2026-10-20T09:06:00.000Z"} -->
## `fn main()`

Exactly one per binary.
