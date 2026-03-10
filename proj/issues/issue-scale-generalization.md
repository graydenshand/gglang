# Generalize the scale architecture

## Problem

`ScaleXContinuous` (`plot.rs` lines 443-527) and `ScaleYContinuous` (`plot.rs` lines 532-614) are ~95% identical. The only differences are: which axis they operate on, which `WindowSegment` method they call for position mapping, and the direction of axis line rendering.

This duplication will compound significantly as new scales are added. A full ggplot2-compatible implementation would need:
- `ScaleXDiscrete`, `ScaleYDiscrete`
- `ScaleColorContinuous`, `ScaleColorDiscrete`
- `ScaleSizeContinuous`, `ScaleSizeDiscrete`
- `ScaleAlphaContinuous`

Each would re-implement the same transform/fit/map/render boilerplate.

Additionally, `Mapping` is an enum (`Mapping::X(String)` / `Mapping::Y(String)` in `plot.rs` lines 333-336) that hardcodes only X and Y. It cannot express mappings for color, size, shape, or alpha without adding a new variant per aesthetic.

## Suggested approach

Separate the three concerns currently tangled in each scale struct:

1. **Scale transform logic** — how data values map through the scale (e.g., linear interpolation, log transform, ordinal lookup). This is already partially in `ContinuousNumericScale` (`transform.rs`); the goal is to make this composable.

2. **Aesthetic binding** — which visual channel the scale serves (horizontal position, color, point size, etc.). Rather than encoding this in the type name (`ScaleXContinuous`), it becomes a parameter.

3. **Visual rendering** — how the scale is drawn (axis lines + ticks + labels for position scales, color bars for color scales, etc.). This is scale-type-specific but should not be duplicated between X and Y.

For `Mapping`, replace the enum with a struct:
```rust
struct Mapping {
    aesthetic: AestheticId,  // "x", "y", "color", "size", etc.
    variable: String,         // column name in PlotData
}
```

## Key files

- `src/plot.rs` — `ScaleXContinuous`, `ScaleYContinuous`, `Scale` trait, `Mapping` enum, `Aesthetic` traits

## Open questions

- Should scale transforms be generic over data types, or should `PlotParameter` become more strongly typed to encode the pipeline stage?
- How should the scale rendering (axis vs. legend) be selected — by the aesthetic binding, or by explicit configuration?
