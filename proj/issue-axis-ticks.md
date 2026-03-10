# Axis tick marks and labels

## Problem

Both `ScaleXContinuous::render()` and `ScaleYContinuous::render()` in `plot.rs` currently emit only a single label at the maximum data value (lines 496-504 and 584-591 respectively). There are no tick marks and no labels at intermediate values. The axes are present but carry no quantitative information beyond the axis line itself.

## Suggested approach

### 1. Add `ticks(n: usize) -> Vec<f64>` to `ContinuousNumericScale` (`transform.rs`)

Produce `n` evenly spaced values across the scale's `[min, max]` range:

```rust
pub fn ticks(&self, n: usize) -> Vec<f64> {
    (0..n).map(|i| self.min + (i as f64 / (n - 1) as f64) * self.span()).collect()
}
```

### 2. Update `ScaleXContinuous::render()` and `ScaleYContinuous::render()` (`plot.rs`)

Replace the single max-value label with a loop over `data_scale.ticks(5)`:

- For each tick value:
  - Map the tick value to NDC position via `data_scale.map_position(&NDC_SCALE, tick)`
  - Emit a short `Rectangle` perpendicular to the axis (e.g. 1px wide × 8px tall for X axis ticks)
  - Emit a `Text` label with the tick value formatted to a reasonable precision

For the X axis, ticks hang below the axis line; for Y axis, ticks extend to the left.

### 3. Label formatting

Round tick values to a reasonable number of significant figures. For the POC, formatting as integers when values are whole numbers and to 1 decimal place otherwise is sufficient. A proper "nice numbers" algorithm (Wilkinson's or similar) is a future improvement.

## Key files

- `src/transform.rs` — add `ticks()` to `ContinuousNumericScale`
- `src/plot.rs` — update `ScaleXContinuous::render()` (lines 479-507) and `ScaleYContinuous::render()` (lines 568-594)

## Open questions

- Should tick count be configurable per scale, or fixed at 5 for now?
- The Y axis label positions currently use `Unit::Percent` for horizontal position, which places them at the left edge of the window segment. Once a layout tree exists, labels should sit in a dedicated axis gutter. For the POC, approximate placement is acceptable.
