# Roadmap

## Completed
1. ~~**Plot labels**~~ -- Add plot title, axis ticks, axis labels, and caption
2. ~~**Scale generalization**~~ — Unified into `ScalePositionContinuous`; `Aesthetic`, `AestheticFamily`, `Mapping` as enums/structs.
3. ~~**Color segmentation**~~ — `ScaleColorDiscrete`, `Aesthetic::Color`, mixed-type CSV, legend rendering.
4. ~~**Timeseries traces**~~ — `GeomLine` with `group` aesthetic, per-group coloring.
5. ~~**Layout tree**~~ — `PlotRegion`, `LayoutNode`, `SizeSpec`, `SplitAxis`, `slice_x`/`slice_y` subdivision.
6. ~~**Shader architecture**~~ — View transform uniform, instanced SDF points, miter-join polylines, three render pipelines.
7. ~~**Render backend abstraction (phase 1)**~~ — `shape.rs` and `layout.rs` fully backend-agnostic; GPU types confined to `frame.rs`.
8. ~~**PlotData typing**~~ — `RawColumn`/`MappedColumn`/`AesData`/`ResolvedData` split; compile-time pipeline guarantees.
9. ~~**Split plot module**~~ — Break `plot.rs` into `geom.rs`, `scale.rs`, `aesthetic.rs` for navigability.
10. ~~**Geom attribute syntax**~~ — Grammar + compiler support for `GEOM TYPE { key=value, ... }` blocks.
11. ~~**Hardcoded aesthetics**~~ — Set constant aesthetic values on a layer, e.g. `GEOM POINT { color="#0000FF" }`.
12. ~~**Per-layer mappings**~~ — Override plot-level defaults on individual layers, e.g. `GEOM LINE { color=:region }`.
13. ~~**Faceting**~~ — `FACET WRAP :var` and `FACET GRID ROWS :r COLS :c` with scale freedom controls.
14. ~~**SVG/PNG export**~~ — `src/svg.rs` + `src/png.rs` via `resvg`. `--output <path>` flag on the `plot` binary. Pixel-coordinate methods (`px_x/y/width/height`) on `WindowSegment`.
15. ~~**Error handling**~~ — Replace `.unwrap()`/`.expect()` panics with `Result` propagation and structured errors. Covers parser, compiler, data loading, and render paths.
16. ~~**ScalePositionDiscrete**~~ — Categorical position scale; auto-detects string columns; `SCALE X/Y DISCRETE` syntax overrides numeric columns. Unblocks `GeomBar`.
17. ~~**Geom bar**~~ — `GeomBar` with stat count/identity, stack/dodge positioning, `Fill` aesthetic, `ScaleColorDiscrete` parameterized for Fill. Position adjustment grammar (`GEOM BAR DODGE`).
18. ~~**Alpha aesthetic**~~ — `Aesthetic::Alpha`, `ScaleAlphaContinuous` (numeric → [0.1, 1.0]), constant injection, all geoms (point, line, bar) consume alpha.

## Current sprint

- **Snapshot testing** (`issues/snapshot_testing.md`) — SVG-based regression testing framework.

## Backlog — Features

- **Geom text** (`stories/geom_text.md`) — Annotate points with text labels. Maps `label` aesthetic to positioned text.
- **Continuous color scale** (`stories/scale_color_continuous.md`) — Map numeric variable to color gradient. Prerequisite for heatmaps.
- **Geom histogram** (`stories/geom_histogram.md`) — Bin continuous data and plot distribution.
- **Geom tile** (`stories/geom_tile.md`) — Heatmap / tile plot with two categorical axes and fill color.
- **Shape aesthetic** (`stories/shape_aesthetic.md`) — Map categorical variable to point marker shape. Requires new SDF functions in the point shader.
- **Size aesthetic** (`stories/size_aesthetic.md`) — Map numeric variable to point radius for bubble charts.
- **Multi-legend layout** (`stories/multi_legend.md`) — Stack multiple legends (color, shape, size) in the legend region. Stress-tests layout.
- **Polar coordinates** (`stories/coord_polar.md`) — `COORD POLAR` transform for pie charts, radar plots, rose diagrams.
- **Zoom & pan** (`stories/zoom_pan.md`) — Activate the view transform uniform with mouse/trackpad input.
- **Nested layouts / sparklines** (`stories/nested_layouts_sparklines.md`) — Low priority. Nest mini-plots inside layout cells.

## Backlog — Infrastructure

- **Theme overrides** (`stories/theme_overrides.md`) — `THEME { key=value }` inline overrides and `THEME FILE "path"` references, stackable so a base company theme can be extended per-plot.
- **Log scale** (`stories/log_scale.md`) — `ScaleLogContinuous` + `SCALE` statement grammar support.

## Backlog — Integrations

- **Python bindings** (`stories/python_bindings.md`) — PyO3/maturin, DataFrame ingestion, SVG output for notebooks.
- **WASM target** (`stories/wasm_target.md`) — Browser embedding via WebGPU/WebGL.
- **Data-viz studio** (`stories/data_viz_studio.md`) — Local-first app combining DuckDB + gglang. Separate crate consuming gglang as a library.
