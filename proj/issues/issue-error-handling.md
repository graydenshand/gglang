# Replace panics with Result propagation

The codebase has many `.unwrap()` and `.expect()` calls, particularly in `ast.rs` (parser) and `compile.rs`. These cause hard crashes on malformed input instead of producing useful error messages.

## Scope

1. **Parser errors** (`ast.rs`): wrap Pest parse failures in a structured `ParseError` with line/column info and a human-readable message
2. **Compilation errors** (`compile.rs`): return `Result<Blueprint, CompileError>` instead of panicking on unknown aesthetics, missing mappings, etc.
3. **Data errors** (`data.rs`): return `Result` from CSV loading for missing files, malformed rows, type mismatches
4. **Render errors** (`plot.rs`): `Blueprint::render()` returns `Result<PlotOutput, RenderError>` for cases like missing required aesthetics, empty data, scale domain issues

## Approach

- Define a `GglangError` enum (or separate error types per module) in a new `src/error.rs`
- Propagate with `?` up to `main()` / `bin/plot.rs` where errors are formatted for the user
- Use `thiserror` or manual `Display` impls — keep dependencies minimal

## Status

Not started.
