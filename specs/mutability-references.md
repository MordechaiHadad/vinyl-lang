# Mutability & References

## Variable Declaration

```
let name: Type = value;
let name = value;        // inferred
let mut name: Type = value;
```

Variables declared at module scope (outside functions) are always **immutable** — the `mut` keyword is not permitted on globals.

## Variable Binding & Mutability

- **Immutable by Default:** Variable declarations using `let` are immutable.
- **Explicit Mutability:** To allow reassignment, `mut` must be explicitly provided.

```
let x = 10;     # Immutable
let mut y = 20; # Mutable
```

## Value Assignment & Copy-on-Write (CoW)

- **Immutable Assignment (`let y = x`):** If `x` is immutable, `y` points to `x`'s underlying memory slot internally via static Copy-on-Write (CoW).
- **Mutable Assignment (`let y = x` when `x` or `y` is `mut`):** Performs an explicit value copy.

```vinyl
let x = 10;
let y = x; # Here we will either do the following: if value of x is smaller than the size of a pointer (16 bytes on 64 bits/8 bytes on 32 bits) we will simply copy the value outright, otherwise it will be a literal reference.

let x = 10;
let mut y = x; # Here we will use a similar strategy but with static CoW (compiler time instead of runtime) instead of a regular reference, so if you later decide to do the following:
y *= 69; # We will copy the data of x and then modify that data. (Though technically this is smaller than 16 bytes just for demonstration)
```

## Reference Semantics (`&`)

References are explicit address bindings that avoid copying memory.

### One-Way Mutability (Read-Only Reference)

```
let mut x = 10;
let y = &x; # 'y' references 'x'

x = 69;     # Allowed: Both 'x' and 'y' now evaluate to 69
y = 420;    # Compile Error: 'y' is not a mutable reference

# x doesn't have to be mutable in order to be referenced like so:
let x = 10;
let y = &x; // Allowed
```

### Two-Way Mutability (Read/Write Reference)

```
let mut x = 10;
let mut y = &x; # 'y' is a mutable reference to 'x'

y = 420;    # Allowed: Updates the value at 'x' to 420 (x and y are both 420)
```

### Value Writing vs. Reference Re-binding

- **Value Store (`y = z`):** Copies the value of `z` into the memory target currently referenced by `y`.
- **Re-pointing (`y = &z`):** Updates `y` to point to the address of `z`.

```
let mut x = 10;
let mut z = 69;
let mut y = &x;

y = z;  # Overwrites 'x' with 69 (x = 69, y = 69)
y = &z; # 'y' now points to 'z'
```

## Function Semantics & Parameter Passing

### References Are Exclusively for Side Effects (`&T`)

Passing a reference (`&`) into a function requires a mutable variable argument. This guarantees functions cannot introduce implicit side effects on immutable data.

```
fn modify(param: &int) {}

let x = 10;
let mut y = 10;

modify(&x); # Compile Error: Cannot pass immutable binding as '&' reference
modify(&y); # Allowed
```

### Pass-by-Value Defaults (`T`)

When passing arguments by value (`param: int`), immutable data is automatically passed via internal read-only pointers for zero-copy efficiency.

```
fn read_only(param: int) {}

read_only(x); # Allowed (Zero-copy internal pointer)
read_only(y); # Allowed (Value copy)
```

### Reference Return

Vinyl functions can return references (made possible by the GC).

```
fn get_ref(): &int {} # Returns a mutable reference
```

## Dynamic Data

Interior pointers into elements of heap allocated data are strictly disallowed to prevent memory invalidation.

```
let x = [1, 2, 3, 4];

let y = &x[0]; # If x is a fixed sized array this will pass otherwise you will get: Compile Error: Cannot take reference to index element 
let y = x[0];  # Allowed (Value copy)
```

## Lexical Scope & Lifetime Boundaries

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