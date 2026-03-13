As an analyst, I want to override the plot-level default mappings on individual layers, so I can compose multiple geoms with different aesthetics on the same plot.

Currently, all layers inherit the plot-level `MAP` and there's no way to customize per-layer. The grammar and compiler need to support per-geom mapping overrides.

# Examples

Overlay a line on a scatterplot, with color only on the line layer:

```gg
MAP x=:year, y=:sales
GEOM POINT
GEOM LINE { color=:region }
```

Two point layers pulling different Y columns onto the same axes:

```gg
MAP x=:year
GEOM POINT { y=:revenue }
GEOM POINT { y=:cost }
```

# Implementation notes

- Extend `grammar.pest`: add an optional `{ mapping, ... }` block after `geometry_type`
- Extend `AstGeom` / `GeomStatement` in `ast.rs` to carry optional per-layer mappings
- In `compile.rs`, merge per-layer mappings on top of plot-level defaults when building each `Layer`
- The `{ }` block syntax aligns with the hardcoded aesthetics story — both need per-geom attribute parsing

# Status

Not started.
