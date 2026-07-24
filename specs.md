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
- **Literals**: Default types are assigned — `int` literals default to `int` (`int64`), `float` to `float` (`float64`), `bool` to `bool`, `string` to `string`. Type annotations on the binding or function signature override these defaults.
- **Binary ops**: Both operands must unify. Comparison and logical operators (`==`, `<`, `&&`, `||`, etc.) return `bool`. Arithmetic operators return the operand type.
- **Calls**: Function call arguments are unified against the callee's parameter types. Return type is determined from the callee's signature.
- **Annotations**: Type annotations are unified against inferred types — a mismatch produces a type error.
- **Unresolved type variables**: `Var` types that remain unresolved after checking are kept as-is rather than being silently defaulted.

## Syntax

### Primitive Types

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

### Literals
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
Logical:    &&  ||  !  not    (also `and`, `or`)
Unary:      - (negate)  ! (not)  not (not)
Bitwise:    &  |  ^  ~  <<  >>
Assignment: =  +=  -=  *=  /=  %=  &=  |=  ^=  <<=  >>=
Range:      ..  ..=  (exclusive/inclusive)
Access:     .  ?.  (optional chaining on Option)
Pipe:       |>  |>>  (forward pipe: first / last argument)
Error prop: ?  (unwraps Result/Option, propagates error/None)
Unwrap:     ??  (unwrap `Option`/`Result` with a fallback or early return)
```

Unary `-` negates a numeric value. Unary `!` / `not` perform logical NOT on a `bool` -- no truthiness coercion.

No `++` or `--`. Use `+= 1` / `-= 1`.

### Pipe Operators

The pipe operators `|>` and `|>>` provide a forward-pipeline syntax for chaining function calls. They are syntactic sugar — the compiler desugars them into nested function calls at parse time. No new AST node, HIR node, or runtime representation is introduced.

- `|>` pipes the left operand as the **first** argument to the function on the right.
- `|>>` pipes the left operand as the **last** argument to the function on the right.

```
x |> f()          // → f(x)
x |> f(a, b)      // → f(x, a, b)
x |> f            // → f(x)          (bare identifier treated as function call)
x |>> f(a, b)     // → f(a, b, x)
x |>> f()         // → f(x)
5 |> int_func()   // → int_func(5)    (works with literals too)
```

**Chaining** — multiple pipes associate left-to-right:

```
x |> f |> g       // → g(f(x))
x |> f(a) |> g(b) // → g(f(x, a), b)
```

**Type inference** — the first function in a pipe chain infers its type normally. The piped result is then unified with the parameter of the next function, propagating the type through the chain. Type errors are reported at the point of mismatch in the chain.

The right side of a pipe must be a function call (with or without arguments) or a bare identifier referencing a function. Piping into non-callable expressions produces a compile error.

### Variable Declaration

```
let name: Type = value;
let name = value;        // inferred
let mut name: Type = value;
```

Variables declared at module scope (outside functions) are always **immutable** — the `mut` keyword is not permitted on globals.

### Mutability & References

#### Variable Binding & Mutability

- **Immutable by Default:** Variable declarations using `let` are immutable.
- **Explicit Mutability:** To allow reassignment, `mut` must be explicitly provided.

```
let x = 10;     # Immutable
let mut y = 20; # Mutable
```

#### Value Assignment & Copy-on-Write (CoW)

- **Immutable Assignment (`let y = x`):** If `x` is immutable, `y` points to `x`'s underlying memory slot internally via static Copy-on-Write (CoW).
- **Mutable Assignment (`let y = x` when `x` or `y` is `mut`):** Performs an explicit value copy.

```vinyl
let x = 10;
let y = x; # Here we will either do the following: if value of x is smaller than the size of a pointer (16 bytes on 64 bits/8 bytes on 32 bits) we will simply copy the value outright, otherwise it will be a literal reference.

let x = 10;
let mut y = x; # Here we will use a similar strategy but with static CoW (compiler time instead of runtime) instead of a regular reference, so if you later decide to do the following:
y *= 69; # We will copy the data of x and then modify that data. (Though technically this is smaller than 16 bytes just for demonstration)
```

#### Reference Semantics (`&`)

References are explicit address bindings that avoid copying memory.

##### One-Way Mutability (Read-Only Reference)

```
let mut x = 10;
let y = &x; # 'y' references 'x'

x = 69;     # Allowed: Both 'x' and 'y' now evaluate to 69
y = 420;    # Compile Error: 'y' is not a mutable reference

# x doesn't have to be mutable in order to be referenced like so:
let x = 10;
let y = &x; // Allowed
```

##### Two-Way Mutability (Read/Write Reference)

```
let mut x = 10;
let mut y = &x; # 'y' is a mutable reference to 'x'

y = 420;    # Allowed: Updates the value at 'x' to 420 (x and y are both 420)
```

##### Value Writing vs. Reference Re-binding

- **Value Store (`y = z`):** Copies the value of `z` into the memory target currently referenced by `y`.
- **Re-pointing (`y = &z`):** Updates `y` to point to the address of `z`.

```
let mut x = 10;
let mut z = 69;
let mut y = &x;

y = z;  # Overwrites 'x' with 69 (x = 69, y = 69)
y = &z; # 'y' now points to 'z'
```

#### Function Semantics & Parameter Passing

##### References Are Exclusively for Side Effects (`&T`)

Passing a reference (`&`) into a function requires a mutable variable argument. This guarantees functions cannot introduce implicit side effects on immutable data.

```
fn modify(param: &int) {}

let x = 10;
let mut y = 10;

modify(&x); # Compile Error: Cannot pass immutable binding as '&' reference
modify(&y); # Allowed
```

##### Pass-by-Value Defaults (`T`)

When passing arguments by value (`param: int`), immutable data is automatically passed via internal read-only pointers for zero-copy efficiency.

```
fn read_only(param: int) {}

read_only(x); # Allowed (Zero-copy internal pointer)
read_only(y); # Allowed (Value copy)
```

##### Reference Return

Before I decided that vinyl functions cannot return references, but due to the fact that we add GC, there is no problem with this.

```
fn get_ref(): &int {} # Returns a mutable reference
```

#### Dynamic Data

Interior pointers into elements of heap allocated data are strictly disallowed to prevent memory invalidation.

```
let x = [1, 2, 3, 4];

let y = &x[0]; # If x is a fixed sized array this will pass otherwise you will get: Compile Error: Cannot take reference to index element 
let y = x[0];  # Allowed (Value copy)
```

#### Lexical Scope & Lifetime Boundaries

References cannot escape their lexical scope. Attempting to point an outer-scope reference to an inner-scope variable is illegal; only value copies are permitted across scope depth boundaries.

```
{ # Scope Depth 0
    let mut x = 10;
    { # Scope Depth 1
        let y = 69;
        x = &y; # Compile Error: Cannot reference inner scope variable 'y'
        x = y;  # Allowed (Value copy)
    }
}
```

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

The last expression in a function body is returned implicitly (like Rust). Explicit `return` is also allowed.

```vinyl
fn add(a: int32, b: int32): int32 {
    a + b                          # implicit return
}

fn add_explicit(a: int32, b: int32): int32 {
    return a + b;                  # explicit return (also valid)
}

fn greet(name: string): string {
    let greeting = f"Hello, {name}";
    greeting                       # implicit return of greeting value
}

fn log(message: string): unit {
    print(message);                # no return value needed for unit
}

fn log_short(message: string) {
    print(message);                # return type defaults to unit
}
```

**Default arguments**: parameters can have default values.

```
fn greet(name: string, greeting: string = "Hello"): string {
    return f"{greeting}, {name}";
}

greet("World");                         // "Hello, World"
greet("World", "Hey");                  // "Hey, World"
```

### Structs

#### Unboxed Value Semantics

User-defined structs are intended to be unboxed value types by default.

- **Storage:** Values are laid out contiguously in stack slots or inside parent layouts. They do not carry GC headers or implicit heap-pointer indirections.
- **Array Layout:** An array of type `[N]T` is one contiguous block of `N * sizeof(T)` bytes.
- **Current implementation:** Struct declarations, HIR registration, field layout, and field offset resolution are implemented. Struct literal construction and complete struct value operations are not implemented yet.

#### Field Reordering

By default, struct fields are reordered at compile time to reduce interior padding.

1. **Alignment Sorting:** Fields are sorted by descending alignment requirements (`8 -> 4 -> 2 -> 1` bytes).
2. **Offset Calculation:** Each field offset is computed as `align_to(current_offset, field_alignment)`.
3. **Struct Padding:** Total size is rounded up to the maximum field alignment.

```vinyl
# Source definition:
struct Character {
    active: bool,
    id: uint64,
    hp: uint32,
}

# Compiled layout:
# [ id (8B) | hp (4B) | active (1B) | padding (3B) ]
```

#### FFI Interoperability (`@repr_c`)

Annotating a struct with `@repr_c` disables field reordering and preserves source declaration order for C-compatible layouts.

#### ABI Parameter Passing Rules

The intended ABI preserves value semantics while avoiding unnecessary copies:

- **Small structs (`<= 16` bytes):** Passed directly in CPU registers where the target ABI permits.
- **Large structs (`> 16` bytes):** Lowered to pass-by-reference under the hood. The source-level parameter remains a value.

The Cranelift backend does not implement these complete aggregate parameter and return rules yet.

#### TODO

- [ ] Implement struct literal construction: `Character { active: true, id: 1, hp: 100 }`.
- [ ] Implement tuple-struct construction and field access.
- [ ] Implement aggregate copying, assignment, parameters, and return values.
- [ ] Implement the complete small/large aggregate ABI, including values larger than one machine register.
- [ ] Implement deep equality for structs and tuples.
- [ ] Implement enum layouts and construction for values larger than 8 bytes.
- [ ] Implement enum payload extraction and exhaustive pattern matching.

```
struct Name {
    field: Type,
    field2: Type,
}
```

Field access with `.`. Struct update syntax like Rust is planned: `Name { field: new_value, ..other }`.

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

Enum variants are constructed with `Name::Variant(...)`. Unit variants currently use empty parentheses, for example `Name::Variant()`. Small enum values supported by the current Cranelift backend are packed into an `i64` containing a discriminant and payload, and equality is supported for those values.

### Tuples
```
let pair = (value, value);
let first = pair.0;
```

Tuple literals and numeric field access are currently supported. Tuple-struct construction, aggregate passing/return, and deep tuple equality remain TODO.

### Match
```
match value {
    Pattern => expr,
    Pattern => { block },
    _ => expr,
}
```

Planned to be exhaustive. Patterns will destructure structs, enums, and tuples once match code generation is implemented.

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

## Mutability & Reference TODOs

- [ ] Implement Copy-on-Write for immutable bindings such as `let y = x`, including the copy-on-write transition when a mutable write occurs.
- [ ] Implement the specified by-value calling convention: immutable arguments use internal read-only pointers, while mutable arguments are copied by value.
- [ ] Extend reference lifetime validation beyond direct identifier assignments to nested expressions, function arguments, and every reference-producing path.
- [ ] Reject or fully support references to non-identifiers consistently at typecheck time instead of deferring errors to codegen.
- [ ] Reject references to parenthesized array elements, such as `&(x[0])`, with the same diagnostic as `&x[0]`.
- [ ] Support compound assignment through reference parameters, such as `p += 1` for `p: &int32`.
- [ ] Enforce mutability and type rules for array-element assignment during typechecking.

## Editions

Vinyl uses an edition system (like Rust's) to allow syntax and semantics to evolve without breaking existing code.

- Single-file scripts use the latest stable edition automatically.
- Projects with a `vinyl.toml` config file declare the edition in that file (e.g. `edition = "2025"`). The package manager or user sets this; no default fallback.
- The default edition is the latest stable edition.
- Editions can change grammar rules, keywords, operator precedence, and standard library behavior.
- The compiler always supports at least the two most recent editions.
