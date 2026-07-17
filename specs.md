# Vinyl Language Spec

## Philosophy
- Rust modern features (algebraic types, pattern matching, traits, enums, no null)
- C# ergonomics (GC)
- Zig comptime
- Python ease of use (script mode)

## Architecture

File extension: `.vnl`

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
- Source -> Tree-sitter (CST) -> CST Lowering (AST) -> HIR -> Type Checker -> MIR -> Cranelift JIT / LLVM AOT

### Type Inference

Vinyl uses **Hindley-Milner** type inference with unification-based constraint solving.

The current implementation uses a non-generic HM variant (no `let`-polymorphism yet). Each top-level function is checked against its declared or inferred types:

- **Unification**: Terms are unified via standard Robinson's algorithm with an occurs check to prevent recursive types.
- **Scope**: Lexical scoping with push/pop scope management. Each `if` branch and block body gets its own scope.
- **Literals**: Default types are assigned — `int` literals default to `int32`, `float` to `float64`, `bool` to `bool`, `string` to `string`.
- **Binary ops**: Both operands must unify. Comparison and logical operators (`==`, `<`, `&&`, `||`, etc.) return `bool`. Arithmetic operators return the operand type.
- **Calls**: Function call arguments are unified against the callee's parameter types. Return type is determined from the callee's signature.
- **Annotations**: Type annotations are unified against inferred types — a mismatch produces a type error.
- **Unresolved type variables**: `Var` types that remain unresolved after checking are kept as-is rather than being silently defaulted.

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
r"raw string with no \escape sequences"
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
Logical:    &&  ||  !        (also `and`, `or`)
Bitwise:    &  |  ^  ~  <<  >>
Assignment: =  +=  -=  *=  /=  %=  &=  |=  ^=  <<=  >>=
Range:      ..  ..=  (exclusive/inclusive)
Access:     .  ?.  (optional chaining on Option)
Error prop: ?  (unwraps Result/Option, propagates error/None)
Unwrap:     ??  (unwrap `Option`/`Result` with a fallback or early return)
```

No `++` or `--`. Use `+= 1` / `-= 1`.

### Variable Declaration

```
let name: Type = value;
let name = value;        // inferred
let mut name: Type = value;
```

Variables declared at module scope (outside functions) are always **immutable** — the `mut` keyword is not permitted on globals.

### Comments
```
# line comment
/* block comment */
```

### Attributes
```
@inline
@doc("documentation text")
@derive(Debug, Clone)
```

Attributes are metadata annotations placed before definitions. Syntax: `@name` or `@name(expr, ...)`. They have no effect on runtime semantics — the compiler uses them for code generation hints, documentation, or derive macros.

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

Parameters are immutable by default (`mut` keyword to allow mutation). No return type = unit return.

**Default arguments**: parameters can have default values.

```
fn greet(name: string, greeting: string = "Hello"): string {
    return f"{greeting}, {name}";
}

greet("World");                         // "Hello, World"
greet("World", "Hey");                  // "Hey, World"
```

### Structs
```
struct Name {
    field: Type,
    field2: Type,
}
```

Field access with `.`. Struct update syntax like Rust: `Name { field: new_value, ..other }`.

**Field puns**: when a variable name matches a struct field name, you can omit the value.

```
let name = "vinyl";
let age = 1;
let user = User { name, age };  // equivalent to User { name: name, age: age }
```

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

The **`??` operator** provides syntax sugar for `unwrap_or_else`:

```
let value = optional_value ?? "default";          // unwrap or default
let value = fallible_result ?? return Err("fail"); // unwrap or early return
let value = fallible_result ?? break;              // unwrap or break out of loop
```

`??` is a binary operator. The left operand is the `Option`/`Result`. The right operand is a value of the inner type (for defaults) or a control-flow expression (`return`, `break`, `continue`). When the right operand is a control-flow expression, the compiler rewrites it to an early exit from the enclosing function/loop.

### Implicit Main

Top-level statements are automatically bundled into a synthetic execution block. No `fn main` wrapper required — like Python.

If a `fn main()` is also present, the compiler treats top-level statements as setup/initialization code and appends a call to `main()` at the end of the execution block. No boilerplate `if __name__ == "__main__"` check.

```
// entry.vnl
print("hello world");            // runs in synthetic main

fn main() {                      // called at the end of synthetic main
    println("done");
}
```
Output:
```
hello world
done
```

### Arrays & Vectors

`[value, value, ...]` creates a **Vec<T>** (heap-allocated, growable). To create a fixed-size array, provide an explicit size in the type annotation:

```
let vec = [1, 2, 3];                // Vec<int32>
let arr: [int32; 3] = [1, 2, 3];   // array (fixed-size, stack)
let arr = [1, 2, 3] as [int32; 3]; // array via cast
```

Access with `arr[index]`. Arrays are fixed-size on stack. Vectors are heap-allocated dynamic lists (from std crate).

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

## Editions

Vinyl uses an edition system (like Rust's) to allow syntax and semantics to evolve without breaking existing code.

- Single-file scripts use the latest stable edition automatically.
- Projects with a `vinyl.toml` config file declare the edition in that file (e.g. `edition = "2025"`). The package manager or user sets this; no default fallback.
- The default edition is the latest stable edition.
- Editions can change grammar rules, keywords, operator precedence, and standard library behavior.
- The compiler always supports at least the two most recent editions.
