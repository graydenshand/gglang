# Log scale

Add `ScaleLogContinuous` for data that spans multiple orders of magnitude (e.g. population, market cap, frequency distributions).

## Design

- Implements `Scale` trait
- Applies `log10` (or configurable base) transform before linear interpolation
- Nice ticks at powers of the base (10, 100, 1000, ...) with optional minor ticks at 2×, 5× intervals
- Domain must be positive — error on zero/negative values

## Language syntax

```gg
MAP x=:year, y=:market_cap
GEOM POINT
SCALE Y_LOG
```

This requires parsing `SCALE` statements in the grammar — currently not implemented.

## Grammar addition

```pest
scale_statement = { "SCALE" ~ scale_type }
scale_type = { "X_CONTINUOUS" | "Y_CONTINUOUS" | "X_LOG" | "Y_LOG" | "COLOR_DISCRETE" }
```

## Dependencies

- The grammar needs `scale_statement` support (could be its own small issue, or bundled here)

## Status

Not started.
