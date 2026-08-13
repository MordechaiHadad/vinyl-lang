# Syntax

## Comments

```
# line comment
/* block comment */
```

## Attributes

```
@inline
@doc("documentation text")
@derive(Debug, Clone)
```

Attributes are metadata annotations placed before definitions. Syntax: `@name` or `@name(expr, ...)`. They have no effect on runtime semantics — the compiler uses them for code generation hints, documentation, or derive macros.

The `@doc` attribute adds documentation to a function, struct, tuple, enum, or type alias. It accepts one string argument, and the documentation is available to the language server through hover:

```
@doc("Adds two numbers")
public fn add(a: int, b: int): int { a + b }
```

Documentation is propagated from the AST into HIR and is shown below the definition signature in hover results, including for imported public definitions.

## Control Flow

```
if condition { } else if condition { } else { }

while condition { }

for element in iterable { }

loop { break; }

return value;
```

## Built-in Functions

```
print(value);
println(value);
println("hello {name}");
println(f"hello {name}");      // f-string: expressions in {}
println(f"{data.access}");     // arbitrary expressions
```

Print to stdout. `println` adds newline.