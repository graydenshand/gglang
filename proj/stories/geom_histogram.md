As an analyst with a continuous variable, I want to plot its distribution as a histogram so I can see the shape, spread, and modality of the data.

# Examples

Basic histogram:

```gg
MAP x=:income
GEOM HISTOGRAM
```

With explicit bin count:

```gg
MAP x=:income
GEOM HISTOGRAM { bins=30 }
```

Colored by group (stacked):

```gg
MAP x=:income, fill=:education_level
GEOM HISTOGRAM
```

Dodged:

```gg
MAP x=:income, fill=:education_level
GEOM HISTOGRAM DODGE
```

# Implementation notes

- **StatBin transform**: the core logic is a `StatBin` stat that takes a continuous X column and produces binned X (bin centers or edges) + Y (counts). Configurable bin count (default: Sturges' rule `ceil(log2(n) + 1)`) or explicit `bins=N` attribute.
- **Relationship to GeomBar**: `GeomHistogram` is essentially `GeomBar` + `StatBin`. It can either be implemented as a thin wrapper around `GeomBar` that injects `StatBin`, or as a standalone geom that produces rectangles directly. The wrapper approach is more compositional.
- **Bin computation**: divide the X domain `[min, max]` into N equal-width bins. Count observations per bin. If `fill` is mapped, count per bin per fill category (for stacked/dodged).
- **Position adjustments**: reuse the existing `Stack`/`Dodge` logic from `GeomBar`.
- **Scale interaction**: X remains continuous (bin edges are numeric positions). Y is count (continuous, auto-generated). The X scale domain should cover the full bin range.
- **Grammar**: `GEOM HISTOGRAM` with optional `{ bins=N }` attribute and optional position (`STACK`/`DODGE`).

# Edge cases

- Empty bins should render as zero-height bars (no gaps in the axis)
- Single-value data (zero variance) should produce a single bin
- Very small datasets might need a minimum bin count

# Dependencies

- `GeomBar` (done) — reuse rendering and position adjustment logic
- `ScalePositionContinuous` (done) — X axis stays continuous unlike categorical bar charts

# Status

Not started.
