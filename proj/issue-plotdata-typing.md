# Strengthen PlotData typing through the pipeline

## Problem

`PlotParameter` (`plot.rs` lines 635-641) is used for data at every stage of the pipeline:
```rust
pub enum PlotParameter {
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),
    UnitArray(Vec<Unit>),
}
```

A raw column of input data (`FloatArray([1.0, 2.0, 3.0])`), a scale-transformed position (`UnitArray([NDC(-0.5), NDC(0.0), NDC(0.5)])`), and any intermediate are all the same type. There's nothing in the type system that prevents passing a raw data array where a post-scale position array is expected, or vice versa.

Similarly, `PlotData` is a `HashMap<String, PlotParameter>` keyed by column name strings, and `MappedData` is a `Vec<(Rc<dyn Aesthetic>, PlotParameter)>`. Moving data through `mapped_data()` and `update_scales()` requires repeated pattern-matching with no compile-time guarantees about which stage's data you're holding.

## Suggested approach

Use distinct types (or a type parameter) to differentiate pipeline stages:

```
RawData         — input column, typed (float/int/string/etc.)
ScaledData      — post-transform, pre-map (e.g., after log transform)
MappedData      — post-map, ready for rendering (e.g., Vec<Unit>)
```

This doesn't need to be complex. Even wrapping `PlotParameter` in newtype structs (`struct RawColumn(PlotParameter)`, `struct MappedColumn(PlotParameter)`) would prevent the most common confusion at compile time.

The stronger version would make `Scale::transform()`, `Scale::map()`, and `Geometry::render()` accept and return distinct types, so the data flow is enforced by the compiler.

## Key files

- `src/plot.rs` — `PlotParameter`, `PlotData`, `MappedData`, `Scale` trait methods, `Geometry::mapped_data()`, `Blueprint::render()`

## Open questions

- Should `PlotParameter` also support string/categorical data (needed for discrete scales and grouping aesthetics like color)?
- Is the `Rc<dyn Aesthetic>` in `MappedData` the right key, or would a simpler `AestheticId` enum/string be more ergonomic?
