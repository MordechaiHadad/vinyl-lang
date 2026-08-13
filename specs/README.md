# Vinyl Language Specs

Readable, topic-scoped design notes for the Vinyl language.

- [Architecture](architecture.md) - philosophy, memory model, modes, dependencies, Rust interop, std library, pipeline
- [Type System](type-system.md) - type inference, primitive types, literals, core enums
- [Operators](operators.md) - operators, pipes, error propagation, arrays
- [Mutability & References](mutability-references.md) - variable declarations, mutability, references
- [Declarations](declarations.md) - functions, structs, enums, tuples, match, impl blocks
- [Syntax](syntax.md) - comments, attributes, control flow, built-in functions
- [Resolver](resolver.md) - import resolution, manifest/script modes, editions

## Maintenance note

These files describe how Vinyl is designed and how we do things. They are
design notes, not a progress log. Implementation status and next steps live
in GitHub issues; when a feature lands or changes direction, update only the
design text here.