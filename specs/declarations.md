# Declarations

## Functions

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

## Structs

```
struct Name {
    field: Type,
    field2: Type,
}
```

### Unboxed Value Semantics

User-defined structs are unboxed value types by default.

- **Storage:** Values are laid out contiguously in stack slots or inside parent layouts. They do not carry GC headers or implicit heap-pointer indirections.
- **Array Layout:** An array of type `[N]T` is one contiguous block of `N * sizeof(T)` bytes.

### Field Reordering

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

### FFI Interoperability (`@repr_c`)

Annotating a struct with `@repr_c` disables field reordering and preserves source declaration order for C-compatible layouts.

### ABI Parameter Passing Rules

The ABI preserves value semantics while avoiding unnecessary copies:

- **Small structs (`<= 16` bytes):** Passed directly in CPU registers where the target ABI permits.
- **Large structs (`> 16` bytes):** Lowered to pass-by-reference under the hood. The source-level parameter remains a value.

### Field Access & Update

Field access with `.`. Struct update syntax like Rust is planned: `Name { field: new_value, ..other }`.

**Field puns**: when a variable name matches a struct field name, you can omit the value.

```
let name = "vinyl";
let age = 1;
let user = User { name, age };  // equivalent to User { name: name, age: age }
```

## Enums

```
enum Name {
    Variant,
    Variant(Type),
    Variant { named: Type },
}
```

Enum variants are constructed with `Name::Variant(...)`. Unit variants use empty parentheses, for example `Name::Variant()`. Small enum values are packed into an `i64` containing a discriminant and payload, and equality is supported for those values.

## Tuples

```
let pair = (value, value);
let first = pair.0;
```

Tuple literals and numeric field access are supported. Tuple-struct construction and aggregate passing/return are TODO; tuple equality is implemented.

## Match

`match` is an expression: the whole match evaluates to the value of the selected arm body. All arm bodies must unify to a single result type, and the match can be assigned or returned like any expression.

```
match value {
    Pattern => expr,
    Pattern => { block },
    _ => expr,
}
```

Patterns:
- `_` wildcard (matches anything)
- `Name` identifier (matches anything, binds the value as an immutable variable)
- `literal` int, bool, char, and string literals
- `(pat, pat, ...)` tuple patterns, destructured positionally
- `TypeName { field, field: pat, ... }` struct patterns; a bare field name binds the field value
- `Type::Variant(pat, ...)` enum variant patterns, destructured positionally (unit variants use `Type::Variant()`)

Arms may carry a guard: `Pattern if condition => expr`. The guard has access to the arm's pattern bindings. A guarded arm never counts toward exhaustiveness, so a guarded match must still end in a non-guarded catch-all arm (`_` or `Name`) or cover every variant/literal.

Exhaustiveness is required: a match with no catch-all arm must cover every enum variant (or both `true` and `false` for a `bool` scrutinee), otherwise the compiler rejects it as non-exhaustive. Guards are evaluated only for arms whose pattern matched; if a guard fails, control falls through to the next arm.

Pattern bindings are immutable and scoped to their own arm (they do not leak into sibling arms or code after the match).

Not supported: `@` binding patterns, or-patterns (`pat | pat`), and nested destructuring of enums inside structs/tuples beyond the direct pattern forms above. Multi-segment scoped paths in patterns (`parent::Type::Variant()`) are not yet supported.

## Impl Blocks

```
impl Type {
    fn method(self, param: Type): ReturnType { }
}

instance.method();
```