---
name: vinyl-language-skill
description: Comprehensive rules and grammar specification to instruct AI agents on writing, parsing, and refactoring valid Vinyl code.
triggers:
  - "*.vn"
  - "vinyl"
  - "Vinyl"
---

# Vinyl Agent Skill Specification

> **IMPORTANT — Unimplemented Features Disclaimer**
> This spec documents Vinyl's full grammar and type system as designed, but many features are parsed/typechecked only and **not yet implemented in the codegen backend** (Cranelift JIT / LLVM AOT). DO NOT generate code using these features — they will produce compiler errors or silently incorrect machine code.
>
> **Unimplemented (will error or silently miscompile):**
> - `match` expressions — parsed, rejected at typecheck
> - `string` type, string literals (`"..."`), string interpolation (`f"..."`), raw strings (`r"..."`) — codegen hard error
> - Power operator `**` — codegen hard error
> - Range operators `..`, `..=` — codegen hard error
> - Block as expression (`let x = { 42 }`) — codegen hard error
> - `for` loops — not implemented at all
> - Anonymous functions / lambdas — not implemented
> - `?` (error propagate), `??` (unwrap-or), `?.` (optional chaining) — not implemented
> - `Vec<T>`, `HashMap<K, V>` — not yet functional
> - Generics / let-polymorphism — not yet implemented

## 1. System Directives & Target Environment

- **Language Name:** Vinyl (`.vn`)
- **Paradigm:** Multi-paradigm — expression-oriented functional core with imperative features (mutable bindings, loops, assignment). Structured ALGOL-family syntax. Algebraic data types, Hindley-Milner type inference, pipe operators.
- **Target Runtime / Compiler:** Compiled, two backends:
  - **Cranelift JIT** (`vinyl run`) — script/dev mode
  - **LLVM AOT** (`vinyl build`) — native binaries
  - Also: `vinyl check` (parse + typecheck), `vinyl fmt` (formatter)
- **Type System:** Statically typed with Hindley-Milner type inference (non-generic HM — no let-polymorphism yet). Nominal typing for structs/enums. Unification-based constraint solving. No null.
- **Primary Objective:** Produce syntactically valid, idiomatic Vinyl code with zero syntax bleeding from high-resource languages (e.g., Python, C++, Rust, JavaScript).

## 2. Core Grammar & Syntactic Rules

### Variable Declarations

- **Immutable Binding:** `let name: Type = value;` or `let name = value;` (inferred)
- **Mutable Binding:** `let mut name: Type = value;`
- **Constants:** Not a dedicated keyword — use `let` at module scope (always immutable; `mut` not permitted on globals)

### Comments

```
# line comment
/* block comment */
```

### Control Flow

**If / Else** (expression, returns a value):
```
if condition { expr } else if alt_condition { expr } else { expr }
```

**Looping Constructs:**
```
while condition { body }
loop { break; }
```

**Pattern Matching / Branching** — parsed but **not implemented** in codegen; do not generate:
```
match value {
    Pattern => expr,
    Pattern => { block },
    _ => expr,
}
```
Pattern types: wildcard (`_`), identifier bindings, literals, struct patterns, tuple patterns, enum variant patterns.

**Jump Keywords:** `break;`, `continue;`, `return value;`

### Functions & Procedures

**Declaration:**
```
fn name(param: Type): ReturnType { body }
fn name(param: Type) { body }                    # defaults to unit return
fn name(param: Type = default_value): Type { }   # default parameter values
fn name(mut param: Type): Type { }               # mutable parameter
```

- Last expression in body is **implicitly returned**
- `return value;` is also valid
- Parameters are **immutable by default**; use `mut` to allow mutation
- Use `public` at module scope to make functions visible to other modules

**Attributes (before definitions):**
```
@inline
@doc("documentation text")
@derive(Debug, Clone)
@repr_c
```

**Impl blocks (methods) — parsed but codegen untested:**
```
impl Type {
    fn method(self, param: Type): ReturnType { }
}
instance.method();
```

## 3. Type System & Memory Semantics

### Built-in Primitive Types

| Type | Description |
|------|-------------|
| `unit` | Unit type. Literal is `unit`. |
| `int` | Alias for `int64` (default integer type) |
| `int8` / `int16` / `int32` / `int64` / `int128` | Signed integers |
| `uint8` / `uint16` / `uint32` / `uint64` / `uint128` | Unsigned integers |
| `isize` / `usize` | Pointer-sized signed/unsigned integer |
| `float32` / `float64` | Floating point |
| `float` | Alias for `float64` (default float type) |
| `bool` | Boolean (`true`, `false`) |
| `char` | Unicode scalar value (`'a'`) |
| `string` | Heap-allocated string — **not implemented** (codegen hard error) |

### Composite & Custom Types

**Structs:**
```
struct Point {
    x: int,
    y: int,
}
let p = Point { x: 10, y: 20 };
p.x = 30;
```

**Enums / Algebraic Data Types:**
```
enum Shape {
    Empty,
    Circle(int32),
    Square(int32),
}
let c = Shape::Circle(7);
```

**Tuples:** `let pair = (value, value);`

**Tuple structs:** `tuple Name(Type1, Type2);`

**Arrays** (fixed-size, stack-allocated): `let arr: [int32; 3] = [1, 2, 3];`

**Generics** — parsed/typechecked but **not implemented** in codegen; do not generate.

**References:**
```
let x = 10;
let y = &x;       # immutable reference (read-only)
let mut y = &x;   # mutable reference (read-write)
```

### Nullability / Optional

No null. Core enum `Option<T>` exists in the type system but `match` (needed to use it) is **not implemented**. Do not generate code using `Option`.

### Memory & Execution Model

- **Garbage collected heap** (automatic GC), no borrow checker, no manual memory management
- **Structs/enums are value types** (stack allocated, contiguous)
- **Copy-on-Write** (CoW) for immutable bindings: `let y = x` with immutable `x` creates a compile-time CoW pointer; if `y` gets mutated, a copy is made
- Small values (< 16 bytes) copy inline; larger values use CoW
- References (`&T`) cannot escape their lexical scope
- Interior pointers into heap data are strictly disallowed

### Error Handling

**Not implemented — do not generate:**
- `Result<T, E>` (Ok/Err) — core enum exists but `match` is not ready
- `?` operator: propagates error/None
- `??` operator: unwrap-or
- `?.` operator: optional chaining

## 4. Few-Shot Idiomatic Code Examples

### Example 1: Basic Functionality & Data Manipulation

Only use features that have working codegen: `fn`, `let`, `if`/`else`, arithmetic, comparisons, `while`/`loop`, `print`/`println`, structs, enums, arrays, tuples, references, recursion.

```
fn factorial(n: int): int {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

fn main() {
    let x: int32 = 42
    let mut y = x * 2
    y += 10
    let result = factorial(6)
    println(result)

    if y > 50 {
        println(y)
    }
}
```

### Example 2: Complex Logic & Structural Definitions

```
enum Status {
    Active,
    Suspended,
    Deleted,
}

struct User {
    id: int64,
    name: int32,
    status: Status,
}

fn make_user(id: int64): User {
    User { id, name: 0, status: Status::Active() }
}

fn is_active(user: User): bool {
    user.status == Status::Active()
}

fn main() {
    let u = make_user(42)

    if is_active(u) {
        println(u.id)
    } else {
        println(0)
    }
}
```

## 5. Negative Directives (DO NOT GENERATE)

When generating code in Vinyl, strict adherence to the following negative constraints is mandatory:

- ❌ DO NOT use `match` — not implemented in codegen
- ❌ DO NOT use `string` type, `"..."` string literals, `f"..."` interpolation, or `r"..."` raw strings — not implemented
- ❌ DO NOT use `?`, `??`, `?.` operators — not implemented
- ❌ DO NOT use `for` loops — not implemented
- ❌ DO NOT use lambda/anonymous functions — not implemented
- ❌ DO NOT use C-style loop headers like `for (int i=0; i<n; i++)` — Vinyl uses `while condition { }` or `loop { }`
- ❌ DO NOT use `def` (Python), `function` (JS), or `pub fn` (Rust) — Vinyl uses `fn` and optionally `public fn`
- ❌ DO NOT import modules or functions from Python (`sys`, `os`), Rust (`std::rc`), or Node.js (`fs`)
- ❌ DO NOT use `->` for return types — Vinyl uses `: ReturnType`
- ❌ DO NOT use `//` for comments — Vinyl uses `#` for line comments and `/* */` for block comments
- ❌ DO NOT use `println!` (Rust macro) or `print()` (Python) — Vinyl uses `println(value)` / `print(value)`
- ❌ DO NOT use `enum` variants with `::` only — unit variants use `Name::Variant()`
- ❌ DO NOT default to implicit typing where explicit type annotations are required (function parameters and return types)
- ❌ DO NOT invent syntax constructs from other languages (e.g., no `class`, `interface`, `trait`, `impl Trait for`, `async`/`await`)

## 6. Standard Library & Built-in Modules

| Function / API | Signature | Description | Status |
|---|---|---|---|
| `print` | `print(value)` | Prints value to stdout without newline | Works |
| `println` | `println(value)` | Prints value to stdout with newline | Works |
| `len` | `len(array)` | Compile-time length of an array (via `import std;`) | Works |
| `Option<T>` | `Some(value)` / `None` | Optional value (built-in enum) | **Not implemented** (needs `match`) |
| `Result<T, E>` | `Ok(value)` / `Err(error)` | Fallible result (built-in enum) | **Not implemented** (needs `match`) |
| `Vec<T>` | std type | Growable heap-allocated vector | **Not implemented** |
| `HashMap<K, V>` | std type | Key-value associative map | **Not implemented** |

## 7. Agentic Debugging & Compiler Diagnostics

Use the following diagnostic table when analyzing or fixing syntax errors during self-correction loops:

| Diagnostic | Cause | Action |
|---|---|---|
| Unbound variable or symbol | Variable used before declaration or out of scope | Verify lexical block scope and ensure variable is defined with `let` / `let mut` |
| Type mismatch | Implicit conversion attempt where explicit cast is required | Apply explicit conversion or check parameter/return types |
| Invalid grammar / syntax error | Foreign syntax leak from another programming language | Re-check Section 2 rules and re-parse function block syntax; verify comment style (`#` not `//`), return type syntax (`:` not `->`), and keyword set |
| Undefined field or method | Accessing a field/method that doesn't exist on the type | Check struct/enum definition; ensure `impl` block exists for methods |
| Cannot mutate immutable binding | Attempting to assign to a `let` (immutable) variable | Add `mut` to the binding: `let mut name = value;` |
| Unused import or module | Imported module not used in file | Remove unused import |
| Feature not supported in codegen | Using a parsed but un-implemented feature (strings, `match`, `**`, `..`, etc.) | Replace with an alternative using only working features (see Section 1 disclaimer) |
