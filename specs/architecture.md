# Architecture

## Philosophy

- Rust modern features (algebraic types, pattern matching, traits, enums, no null)
- C# ergonomics (GC)
- Zig comptime
- Python ease of use (script mode)

## File extension

`.vn`

## Memory

- Automatic GC heap, no borrow checker, no manual memory management
- Structs/enums as value types (stack by default), GC-allocated when assigned to a reference type field or returned from a GC-tracked context

## Modes

- `vinyl run` - Cranelift JIT
- `vinyl build` - LLVM AOT

## Dependencies

| Crate | Version | Role |
|-------|---------|------|
| `tree-sitter` | Parsing / CST (incremental parse, editor integration) |
| `inkwell` | LLVM IR builder for AOT (`vinyl build`) |
| `cranelift-jit` / `cranelift-codegen` | JIT backend for script mode (`vinyl run`) |
| `miette` | Error reporting (graphical diagnostics, diagnostic codes) |
| `clap` | CLI argument parsing |
| `tracing` | Compiler internal logging |

## String interning

Deferred. The parser produces `String` tokens now; adding an interner later is
a local change to the parser's output step - the rest of the compiler sees
`Symbol`/`u32` either way. Options when ready: `asylum`, `stringleton`, `lasso`,
or even `HashMap<String, u32>`.

## Rust Interop

Vinyl can call Rust libraries compiled as `staticlib`/`cdylib` with a
C-compatible interface:

```
extern "C" {
    fn some_rust_function(arg: int32): float64;
}
```

This is the same mechanism Rust uses for FFI. Vinyl's compiler links the
object files together - just declare the signature and link. One-direction for
now: Vinyl calls Rust. No Vinyl-to-Rust export yet.

## Std Library

Core std types (`Vec`, `HashMap`, etc.) are implemented in **Rust** as a native
crate (`vinyl-std`). Higher-level Vinyl-specific APIs can later be written in
Vinyl itself.

## Pipeline

Source -> Tree-sitter (CST) -> CST Lowering (AST) -> HIR -> Type Checker -> MIR -> Cranelift JIT / LLVM AOT