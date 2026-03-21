**Priority: Low**

# Nested layouts and sparklines

As a user with grouped data, I want to see a grid of small plots (sparklines) alongside summary values, so I can spot trends at a glance without building separate plots for each group.

## Key insight

The real primitive here is the ability to **nest a plot inside a layout cell** — not a table geom. A table that only displays text doesn't leverage the engine's strengths (scales, coordinates, geoms). But a layout that can mix text and mini-plots is a genuinely useful capability that also unlocks dashboard-style multi-plot outputs.

## Possible syntax

```gg
LAYOUT TABLE
  COLUMN :ticker
  COLUMN :last_price
  COLUMN SPARKLINE (x=:date, y=:price, group=:ticker)
```

Where `SPARKLINE` renders a mini `Blueprint` into each cell's `WindowSegment`.

## Stepping stone: compact small multiples

Faceting already gets close. `FACET WRAP :ticker` with a compact/minimal theme (strip axes, shrink margins) would produce sparkline-like output without new infrastructure. This could be a useful intermediate step.

## Architectural prerequisites

- A table/grid **layout mode** that subdivides a region into rows and columns of `WindowSegment`s
- The ability to render a sub-`Blueprint` (or at least a mini geom) into an individual cell segment
- Mixing text-only cells and plot cells in the same grid

These primitives would also enable dashboard layouts (multiple independent plots in one output).
