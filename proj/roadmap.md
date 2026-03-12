# Roadmap

## Tier 1 — Expand the grammar of graphics

1. ~~**Scale generalization**~~ (`issues/issue-scale-generalization.md`) — ✅ Done. Unified `ScaleXContinuous`/`ScaleYContinuous` into `ScalePositionContinuous`; `Aesthetic`, `AestheticFamily`, `Mapping` are now enums/structs instead of traits.

2. **Segmenting scatterplots** (`stories/segmenting_scatterplots.md`) — Color ✅ done (`ScaleColorDiscrete`, `Aesthetic::Color`, mixed-type CSV, legend rendering). Shape deferred — requires new `Shape` implementations and shader architecture work. Faceting deferred, requires layout tree.

## Tier 2 — New geom types and deeper architecture

3. ~~**Timeseries traces**~~ (`stories/timeseries_traces.md`) — ✅ Done. `GeomLine` with `group` aesthetic for partitioning series, `LineSegment` shape primitive, `Aesthetic::Group` / `AestheticFamily::Group` (no scale — partitions only). Works with `color` aesthetic for per-group coloring.

4. ~~**Layout tree**~~ (`issues/issue-layout-tree.md`) — ✅ Done. Tree-based layout system with `PlotRegion`, `LayoutNode`, `SizeSpec`, `SplitAxis`. `Unit` and `WindowSegment` moved to `layout.rs` with `slice_x`/`slice_y` subdivision. `Blueprint::render()` returns `PlotOutput` (elements partitioned by region + layout tree). `Frame` resolves layout and projects each region through its own `WindowSegment`. Eliminates all out-of-bounds NDC positioning. Faceting support is now unblocked.

## Tier 3 — Performance and polish (defer)

5. **Shader architecture** (`issues/issue-shader-architecture.md`) — View transform uniform, GPU instancing, SDF point rendering. Defer until data pipeline is solid.

6. **Render backend abstraction** (`issues/issue-render-backend-abstraction.md`) — SVG/PNG export, testability without a GPU device.

7. **PlotData typing** (`issues/issue-plotdata-typing.md`) — Compile-time guarantees across pipeline stages.
