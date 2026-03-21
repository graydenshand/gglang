As an analyst, I want to map a numeric variable to a color gradient so I can visualize continuous variation (e.g. temperature, density, correlation) using color.

### Examples

```gg
MAP x=:longitude, y=:latitude, color=:temperature
GEOM POINT
```

```gg
MAP x=:row, y=:col, color=:value
GEOM TILE
```

### Implementation notes

- **Scale**: `ScaleColorContinuous` implementing `Scale` trait. Maps a numeric domain to a color gradient via interpolation.
- **Default palette**: a sequential gradient (e.g. viridis or blue→red). Diverging palettes (blue→white→red) are a future extension.
- **Legend**: a continuous gradient bar with tick labels at the min/max (and optionally mid), rather than discrete swatches. New legend rendering path.
- **Auto-detection**: `default_scale_for()` should return `ScaleColorContinuous` when the color-mapped column is numeric, and `ScaleColorDiscrete` when it's a string column (current behavior).
- **Grammar**: no new syntax needed — `color=:numeric_var` already parses. Scale selection happens automatically based on column type.

### Dependencies

- Needed by `GeomTile` (heatmaps)
- Gradient legend is a new rendering path in both GPU and SVG backends

### Status

Not started.
