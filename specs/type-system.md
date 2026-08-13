# Type System

## Type Inference

Vinyl uses **Hindley-Milner** type inference with unification-based constraint solving.

The inference is a non-generic HM variant (no `let`-polymorphism yet). Each top-level function is checked against its declared or inferred types:

- **Unification**: Terms are unified via standard Robinson's algorithm with an occurs check to prevent recursive types.
- **Scope**: Lexical scoping with push/pop scope management. Each `if` branch and block body gets its own scope.
- **Literals**: Default types are assigned — `int` literals default to `int` (`int64`), `float` to `float` (`float64`), `bool` to `bool`, `string` to `string`. Type annotations on the binding or function signature override these defaults.
- **Binary ops**: Both operands must unify. Comparison and logical operators (`==`, `<`, `&&`, `||`, etc.) return `bool`. Arithmetic operators return the operand type.
- **Calls**: Function call arguments are unified against the callee's parameter types. Return type is determined from the callee's signature.
- **Annotations**: Type annotations are unified against inferred types — a mismatch produces a type error.
- **Unresolved type variables**: `Var` types that remain unresolved after checking are kept as-is rather than being silently defaulted.

## Primitive Types

| Type | Description |
|------|-------------|
| `unit` | Unit type (no value). `unit` is also a valid expression that evaluates to unit. |
| `int8` | 8-bit signed integer |
| `int16` | 16-bit signed integer |
| `int32` | 32-bit signed integer |
| `int64` | 64-bit signed integer |
| `int128` | 128-bit signed integer |
| `isize` | Pointer-sized signed integer |
| `uint8` | 8-bit unsigned integer |
| `uint16` | 16-bit unsigned integer |
| `uint32` | 32-bit unsigned integer |
| `uint64` | 64-bit unsigned integer |
| `uint128` | 128-bit unsigned integer |
| `usize` | Pointer-sized unsigned integer |
| `float32` | 32-bit floating point |
| `float64` | 64-bit floating point |
| `bool` | Boolean |
| `char` | Unicode scalar value |
| `string` | Heap-allocated string primitive (like C#, not Rust's `String`/`&str` split) |
| `int` | 64-bit signed integer (default int type) |
| `float` | 64-bit floating point (default float type) |

`string` is a reference type (GC-tracked). `unit` is also the default return type when no return type is specified on a function.

## Literals

```
"string literal"
r"raw string with no \escape sequences"
f"interpolation {expr}"
'a'
42       // int (positive only)
3.14     // float (positive only)
true / false
unit     // unit literal (unit type value)
```

Negative numeric literals like `-42` are not a single token -- they are parsed as a unary `-` operator applied to the positive literal. The compiler performs **constant folding** at lowering time: `-42` becomes `Int(-42)` in a single step, so there is no runtime cost for simple negations. Unary `!` and `not` on `bool` literals are folded the same way: `!true` becomes `false`.

**References are explicit with `&T`.** You must annotate reference types so it's always visible whether you're copying a value or referencing heap data. No implicit references.

## Core Enums

```
Option<T>   // Some(value) or None
Result<T, E> // Ok(value) or Err(e)
```

No null. No exceptions. Fallible functions return `Result`. Optional values use `Option`. The `?` operator unwraps or propagates the error/None.