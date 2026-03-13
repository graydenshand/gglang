As an analyst, I want to use polar coordinates so I can create radar charts, rose diagrams, and pie/donut charts by transforming the coordinate system rather than requiring special geometry types.

In ggplot2, `coord_polar()` reinterprets x as angle and y as radius (or vice versa), turning bar charts into pie charts, line charts into radar plots, etc. This is a powerful compositional primitive — one coordinate transform unlocks many chart types without new geoms.

# Examples

Pie chart (bar chart in polar coords):

```gg
MAP x=:category, y=:count
GEOM BAR
COORD POLAR
```

Radar / spider chart:

```gg
MAP x=:metric, y=:score, color=:player
GEOM LINE
COORD POLAR
```

Rose diagram (bars with angular width):

```gg
MAP x=:wind_direction, y=:frequency
GEOM BAR
COORD POLAR START 0
```

# Implementation notes

- **Grammar**: add `coord_statement = { "COORD" ~ coord_type ~ coord_options? }` with `coord_type = { "CARTESIAN" | "POLAR" }`. Options could include `START` (angle offset) and `DIRECTION` (clockwise/counter-clockwise).
- **AST / Compile**: `Blueprint` gains a `coord: CoordSystem` field (default `Cartesian`).
- **Coordinate transform**: a `CoordSystem` trait (or enum with methods) that transforms `(x, y)` unit positions into final positions after scale mapping but before element generation. `Cartesian` is identity. `Polar` maps `(x, y)` → `(y * cos(x * 2π), y * sin(x * 2π))`.
- **Where it fits in the pipeline**: the transform happens inside `Blueprint::render()`, after `Scale::map()` produces `MappedColumn` values but before `Geometry::render()` produces `Element`s. Alternatively, the geom itself could receive the coord system and adapt its rendering — this is closer to how ggplot2 works, where `coord_polar` changes how bars are drawn (wedges instead of rectangles).
- **Axis rendering**: polar coords need a circular grid (concentric circles for y-axis, radial lines for x-axis) instead of the standard rectangular axes. This is a new axis rendering path.
- **Layout**: the data area should be square (or the coord system should handle aspect ratio) to avoid distorted circles.

# Design considerations

- Bars in polar coordinates become wedge/arc shapes — this likely requires a new `Element::Arc` variant or tessellation of arcs into triangles in `frame.rs`
- Lines in polar coords connect points along the angular axis — the polyline tessellator should work as-is once points are in Cartesian screen space after the polar transform
- Points in polar coords just need position remapping — the SDF pipeline handles them fine

# Dependencies

- More useful with `GeomBar` + `ScalePositionDiscrete` (for pie charts)
- Works standalone with `GeomLine` + `GeomPoint` for radar/polar scatter plots

# Status

Not started.
