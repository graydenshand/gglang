# Build a layout tree for plot regions

## Problem

All layout is currently done by a single `WindowSegment::with_margin()` call in `frame.rs` (line 67), which applies a uniform margin to the root window segment. This gives one level of nesting — the plot area inside the window margin — with no further subdivision.

A real plot requires many distinct regions:
- **Data area** — where geoms are drawn (points, lines, bars)
- **X axis gutter** — below the data area, for tick marks and labels
- **Y axis gutter** — to the left of the data area
- **Title region** — above the data area
- **Legend region** — to the right (or other side) of the data area

Faceted plots add another level: a grid of sub-plots, each with their own data areas and (optionally) shared axes.

The current `WindowSegment` concept is exactly right as a leaf node — a rectangular region with coordinate transforms — but there's no tree structure that composes them.

`layout.rs` has a non-compiling stub that identifies the problem but no implementation.

## Suggested approach

Build a layout tree where each node is either:
- A **leaf** `WindowSegment` (the current concept — a renderable region)
- A **split node** that divides its parent region into sub-regions along one axis

Splits can be:
- **Fixed** — a region with a predetermined pixel or percent size (e.g., a title bar with 40px height)
- **Proportional** — regions that share remaining space (e.g., a 70%/30% data/legend split)
- **Content-sized** — a region that shrinks to fit its content (needed for axis labels with variable text width)

This is domain-specific to statistical graphics layout (unlike CSS flexbox) and maps directly onto the existing `ContinuousNumericScale`-based coordinate system. Each split produces new `WindowSegment`s by subdividing NDC and pixel scales.

A grid split (for facets) would be a 2D generalization of the same concept.

## Key files

- `src/layout.rs` — current stub
- `src/shape.rs` — `WindowSegment` (lines 77-180), the leaf node type
- `src/frame.rs` — current one-shot layout via `with_margin()` (line 67)

## Open questions

- Should the layout tree be resolved eagerly (top-down, once, before rendering) or lazily (each node resolves on demand)?
- How should content-sized nodes work — does each geom/text element report its size before layout, or is layout approximate with overflow clipping?
- Does the layout tree need to be serializable (e.g., for debugging or inspection)?
