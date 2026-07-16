# Vinyl Language Spec

## Philosophy
- Rust modern features (algebraic types, pattern matching, traits, enums, no null)
- C# ergonomics (GC)
- Zig comptime
- Python ease of use (script mode)

## Architecture

### Memory
- Automatic GC heap, no borrow checker, no manual memory management
- Structs/enums as value types (stack by default), GC-allocated when assigned to a reference type field or returned from a GC-tracked context

### Modes
- `vinyl run` - Cranelift JIT
- `vinyl build` - LLVM AOT

### Dependencies

| Crate | Version | Role |
|-------|---------|------|
| `tree-sitter` | Parsing / CST (incremental parse, editor integration) |
| `inkwell` | LLVM IR builder for AOT (`vinyl build`) |
| `cranelift-jit` / `cranelift-codegen` | JIT backend for script mode (`vinyl run`) |
| `miette` | Error reporting (graphical diagnostics, diagnostic codes) |
| `clap` | CLI argument parsing |
| `tracing` | Compiler internal logging |

**String interning**: Deferred. Parser produces `String` tokens now; adding an interner later is a local change to the parser's output step - the rest of the compiler sees `Symbol`/`u32` either way. Options when ready: `asylum`, `stringleton`, `lasso`, or even `HashMap<String, u32>`.

### Rust Interop

Vinyl can call Rust libraries compiled as `staticlib`/`cdylib` with a C-compatible interface:

```
extern "C" {
    fn some_rust_function(arg: int32): float64;
}
```

This is the same mechanism Rust uses for FFI. Vinyl's compiler links the object files together - just declare the signature and link. One-direction for now: Vinyl calls Rust. No Vinyl-to-Rust export yet.

### Std Library

Core std types (`Vec`, `HashMap`, etc.) are implemented in **Rust** as a native crate (`vinyl-std`). Higher-level Vinyl-specific APIs can later be written in Vinyl itself.

### Pipeline
- Source -> Lexer -> Parser -> HIR -> Type Checker -> MIR -> Cranelift JIT / LLVM AOT

## Syntax

### Primitive Types

| Type | Description |
|------|-------------|
| `int8` .. `int128` | Signed integers |
| `uint8` .. `uint128` | Unsigned integers |
| `float32` | 32-bit floating point |
| `float64` | 64-bit floating point |
| `bool` | Boolean |
| `char` | Unicode scalar value |
| `string` | Heap-allocated string primitive (like C#, not Rust's `String`/`&str` split) |

`string` is a reference type (GC-tracked).

### Literals
```
"string literal"
f"interpolation {expr}"
'a'
42       // int
3.14     // float64
true / false
```

**References are explicit with `&T`.** You must annotate reference types so it's always visible whether you're copying a value or referencing heap data. No implicit references.

### Core Enums

```
Option<T>   // Some(value) or None
Result<T, E> // Ok(value) or Err(e)
```

No null. No exceptions. Fallible functions return `Result`. Optional values use `Option`. The `?` operator unwraps or propagates the error/None.

### Operators

```
Arithmetic: +  -  *  /  %  **  //
Comparison: ==  !=  <  >  <=  >=
Logical:    &&  ||  !
Bitwise:    &  |  ^  ~  <<  >>
Assignment: =  +=  -=  *=  /=  %=  &=  |=  ^=  <<=  >>=
Range:      ..  ..=  (exclusive/inclusive)
Access:     .  ?.  (optional chaining on Option)
Error prop: ?  (unwraps Result/Option, propagates error/None)
```

No `++` or `--`. Use `+= 1` / `-= 1`.

### Variable Declaration

```
let name: Type = value;
let name = value;        // inferred
let mut name: Type = value;
```

### Comments
```
// line comment
/* block comment */
```

### Control Flow
```
if condition { } else if condition { } else { }

while condition { }

for element in iterable { }

loop { break; }

return value;
```

### Functions
```
fn name(param: Type): ReturnType {
    body
}
```

No return type = unit return. Parameters are immutable by default (`mut` keyword to allow mutation).

### Structs
```
struct Name {
    field: Type,
    field2: Type,
}
```

Field access with `.`. Struct update syntax like Rust: `Name { field: new_value, ..other }`.

### Enums
```
enum Name {
    Variant,
    Variant(Type),
    Variant { named: Type },
}
```

### Tuples
```
let pair = (value, value);
let first = pair.0;
```

### Match
```
match value {
    Pattern => expr,
    Pattern => { block },
    _ => expr,
}
```

Exhaustive. Patterns can destructure structs, enums, tuples.

### Error Propagation
```
fn fallible(): Result<int, Error> {
    let val = may_fail()?;  // unwraps Ok, returns Err on failure
    Ok(val)
}
```

`?` works on both `Result` and `Option`. Same as Rust.

`?.` is optional chaining on `Option`: `option?.field` returns `None` if `option` is `None`, otherwise `Some(value.field)`.

### Entry Point
```
fn main() { }
```

For script mode (`vinyl run`), top-level statements execute directly without `fn main`, like Python.

### Arrays
```
let arr: [Type; length] = [value, value, value];
let first = arr[0];
```

Fixed-size on stack. Dynamic lists via `Vec<T>` in std crate.

### Built-in Functions
```
print(value);
println(value);
println("hello {name}");
println(f"hello {name}");      // f-string: expressions in {}
println(f"{data.access}");     // arbitrary expressions
```

Print to stdout. `println` adds newline.

### Impl Blocks
```
impl Type {
    fn method(self, param: Type): ReturnType { }
}

instance.method();
```
