# CSV data loading

## Problem

There is no data ingestion path. `PlotData` is currently constructed manually in `frame.rs` (lines 55-63) with hardcoded float arrays. To run a real plot, the engine needs to read data from an external source and populate a `PlotData`.

## Suggested approach

Add a new `src/data.rs` module with a `load_csv(path: &str) -> Result<PlotData>` function:

- First row is treated as column headers → becomes `PlotData` keys
- All values parsed as `f64` → stored as `PlotParameter::FloatArray`
- Returns a descriptive error if the file is missing or a value can't be parsed as a float

Use the `csv` crate for parsing (handles quoting, whitespace, etc.) rather than manual splitting. Add it to `Cargo.toml`.

No schema validation is needed at this stage — column types are assumed to be float. String/categorical columns can be added later when discrete scales are implemented.

## Key files

- `src/data.rs` — new module (create)
- `src/lib.rs` — expose `pub mod data`
- `Cargo.toml` — add `csv` dependency
- `src/frame.rs` — hardcoded data at lines 55-63 (to be replaced when wired up in the renderer milestone)

## Open questions

- Should the grammar eventually support inline data literals (e.g. `DATA x = [1, 2, 3]`)? If so, the `PlotData` construction path would be shared between CSV loading and inline parsing. For the POC, external CSV only is sufficient.
- Should column type inference be attempted (e.g. detect integer vs float), or always load as `f64`?
