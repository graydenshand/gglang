As an analyst, I want to facet a plot by a categorical variable so that each group is rendered in its own sub-plot, making comparisons across groups easier.

The layout tree infrastructure is already in place (`LayoutNode`, `SplitAxis`, `WindowSegment::slice_x/slice_y`). This story adds the language syntax, compilation, and render logic to actually split data and produce faceted plots.

# Examples

Basic facet:

```gg
MAP x=:year, y=:sales
GEOM POINT
FACET BY :region
```

Facet with explicit column count:

```gg
MAP x=:year, y=:sales
GEOM LINE { color=:product }
FACET BY :region COLUMNS 3
```

Multi-variable facet:

```gg
MAP x=:year, y=:sales
GEOM POINT
FACET BY :region, :product
```

# Implementation notes

- **Grammar**: add `facet_statement = { "FACET" ~ "BY" ~ facet_vars ~ facet_options? }` with optional `ROWS n` / `COLUMNS n` / `TRANSPOSE`
- **AST**: add `FacetStatement` variant with variable list and layout hints
- **Compile**: store facet spec on `Blueprint`
- **Render** (`Blueprint::render`): partition data by unique values of facet variable(s), render each subset into its own `PlotRegion::DataArea`, generate a grid of sub-plots via `LayoutNode::Split`
- Each facet panel needs its own axis scales (computed per-panel, or shared across panels — shared is more useful for comparison)
- Facet labels (strip text) should appear above or beside each panel

# Dependencies

- Per-layer mappings story (nice to have, not blocking)

# Status

Not started.
