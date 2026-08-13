# Operators

## Operator Reference

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

## Pipe Operators

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

## Error Propagation

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

## Arrays & Vectors

`[value, value, ...]` creates a **Vec<T>** (heap-allocated, growable). To create a fixed-size array, provide an explicit size in the type annotation:

```
let vec = [1, 2, 3];                // Vec<int32>
let arr: [int32; 3] = [1, 2, 3];   // array (fixed-size, stack)
let arr = [1, 2, 3] as [int32; 3]; // array via cast
```

Access with `arr[index]`. Arrays are fixed-size on stack. Vectors are heap-allocated dynamic lists (from std crate).