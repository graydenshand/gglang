As an analyst, I want to map a categorical variable to point shape so I can distinguish groups in a scatterplot without relying on color alone (useful for accessibility and print).

# Examples

```gg
MAP x=:height, y=:weight, shape=:species
GEOM POINT
```

Combined with color:

```gg
MAP x=:height, y=:weight, shape=:species, color=:species
GEOM POINT
```

Hardcoded shape:

```gg
GEOM POINT { shape="triangle" }
```

# Implementation notes

- **Aesthetic**: add `Aesthetic::Shape` and `AestheticFamily::Shape`
- **Scale**: `ScaleShapeDiscrete` — maps unique string values to a palette of shapes (circle, triangle, square, diamond, cross, etc.)
- **Shapes**: need multiple SDF functions in the point fragment shader. Currently only circles are implemented. Each shape variant needs its own signed distance function.
- **Instanced pipeline**: the `PointInstance` struct needs a shape ID field so the fragment shader can switch SDF per point.
- **Legend**: shape legend shows each marker with its label (similar to color legend layout)
- **Grammar**: add `shape` to the `aesthetic` rule in `grammar.pest`

# Dependencies

- Shader work: new SDF functions for each shape variant
- Pairs well with `ScaleColorDiscrete` for redundant encoding (shape + color on same variable)

# Status

Not started. (Split from the original segmenting scatterplots story; color segmentation is done, faceting is its own story.)
