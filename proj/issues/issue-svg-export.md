# SVG/PNG export backend

Phase 2 of the render backend abstraction. The domain model (`Blueprint::render() → PlotOutput`) is fully backend-agnostic. This issue adds a non-GPU rendering path that produces SVG (and optionally PNG via resvg) output.

## Motivation

- Enables use in headless environments (CI, servers, notebooks)
- Produces vector output for publication-quality figures
- Validates the backend abstraction — if SVG export works cleanly, the abstraction is right

## Design

- New module `src/svg.rs` (or `src/export.rs`)
- Takes a `PlotOutput` + root dimensions → produces an SVG string
- Resolves the layout tree the same way `Frame` does (reuse `LayoutNode::resolve`)
- Maps `Element` variants to SVG equivalents:
  - `Rect` → `<rect>`
  - `Text` → `<text>`
  - `Point` → `<circle>`
  - `Polyline` → `<polyline>` or `<path>`
- No wgpu dependency — this module is pure Rust string building (or use an SVG crate)

## CLI integration

```bash
cargo run --bin plot file.gg data.csv --output plot.svg
cargo run --bin plot file.gg data.csv --output plot.png
```

## Status

Not started.
