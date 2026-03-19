# CLAUDE.md

This file contains instructions for Claude for working with this repo.

As you make changes, keep this file up to date with the useful context to help you and others.

`./proj` contains files related to project management; issues are in `./proj/issues` and `./proj/stories` contains user stories.
We'll work off of `./proj/roadmap.md`; as you make changes, keep these up to date (delete tickets that are completed,
update roadmap doc to reflect current status).

## Project Overview

A ggplot2-inspired statistical graphics engine written in Rust, with multiple rendering backends: wgpu for GPU-accelerated interactive rendering, and SVG/PNG for headless file export. The system has two components: a DSL compiler for a language called GQL (Grammar of Graphics Language), and a rendering engine that produces plots from a `Blueprint` specification.

Goals:

- A declarative, language-independent visualization definition (GQL)
- High-performance native rendering suitable for large datasets and interactivity
- Grammar-of-graphics compositional model (layers, scales, aesthetics, stats, facets)
- Deviations from ggplot2: data is decoupled from the plot definition (passed at render time, not embedded); lower-level channel mappings (e.g. `width`/`height` separate from `size`); less reliance on tidy data


## Current state

The parser, compiler, and renderer are connected end-to-end. A `.gg` file and CSV are parsed, compiled into a `Blueprint`, and rendered via wgpu (interactive window) or exported to SVG/PNG (headless). All public API functions return `Result<_, GglangError>` with structured error types via `thiserror`.

Supported features: `GeomPoint` and `GeomLine` with X/Y continuous scales, `group` aesthetic for partitioning line series, color segmentation via `ScaleColorDiscrete` (categorical string column → HSL-spaced colors with legend), axis tick marks/labels, plot titles/captions/axis labels. Faceting supports two modes: `FACET WRAP :var` (wraps panels into an auto-sized grid, one variable, configurable columns) and `FACET GRID ROWS :r COLS :c` (strict rows×cols matrix from one or two variables, with col labels top, row labels right). Both modes support scale freedom controls: `SCALES FREE/FREE X/FREE Y/FIXED`. Free scales in wrap mode are per-panel; in grid mode, free X scales are shared per column and free Y scales are shared per row. The `Scale` trait has `clone_unfitted()` for creating blank scale copies used by free-scale rendering.

CSV loading auto-detects numeric vs. string columns. A tree-based layout system gives each plot region (data area, axis gutters, title, legend, caption, facet labels, facet col/row labels) its own `WindowSegment`. `RegionKey` (compound `PlotRegion` + optional panel index) enables both single-plot and faceted layouts through the same rendering pipeline. The GPU rendering backend uses three separate pipelines: a general pipeline for rectangles/axes/ticks, an instanced SDF pipeline for anti-aliased points, and a miter-join tessellated pipeline for polylines. A view transform uniform (currently identity) unblocks future pan/zoom. The domain modules (`plot.rs`, `aesthetic.rs`, `column.rs`, `geom.rs`, `scale.rs`, `shape.rs`, `layout.rs`) are fully backend-agnostic — all wgpu/winit types are confined to `frame.rs` and `app.rs`, and SVG/PNG export consumes the same `PlotOutput` without any GPU dependency.

## Module map

| File | Role |
|------|------|
| `src/main.rs` | Parser binary — reads `.gg` file, identifies statement types |
| `src/bin/plot.rs` | Renderer binary — interactive wgpu window or headless SVG/PNG export via `--output` |
| `src/lib.rs` | Library root — re-exports modules |
| `src/ast.rs` | AST types and Pest parser: `Program`, `Statement`, `AstAesthetic`, `DataMapping` |
| `src/compile.rs` | Compiles AST `Program` into a `Blueprint` — wires mappings, layers, scales |
| `src/data.rs` | CSV loader — auto-detects numeric (`FloatArray`) vs. string (`StringArray`) columns |
| `src/error.rs` | `GglangError` enum (`Parse`, `Compile`, `Data`, `Render`, `Export`) via `thiserror` |
| `src/aesthetic.rs` | `Aesthetic` enum, `AestheticFamily` enum, `Mapping` struct, `ConstantValue`, `parse_hex_color()` |
| `src/column.rs` | `RawColumn` (input), `MappedColumn` (output), `AesData`, `ResolvedData` — data pipeline types |
| `src/geom.rs` | `Geometry` trait, `GeomPoint`, `GeomLine` — layer rendering implementations |
| `src/scale.rs` | `Scale` trait, `ScalePositionContinuous`, `ScaleColorDiscrete`, `StatTransform`, `default_scale_for()` |
| `src/plot.rs` | `Blueprint` (builder + render orchestration), `Layer`, `PlotData`, `FacetSpec` |
| `src/shape.rs` | Domain-level render primitives: `Rectangle`, `Text`, `PolylineData`, `PointData`, `Element` enum. Backend-agnostic |
| `src/layout.rs` | Layout system: `Unit`, `WindowSegment` (with `slice_x`/`slice_y`, `px_x`/`px_y`), `PlotRegion`, `LayoutNode`, `SizeSpec`, `SplitAxis`, `PlotOutput`. Backend-agnostic |
| `src/theme.rs` | `Theme` struct — colors, fonts, sizing for plot chrome |
| `src/svg.rs` | SVG renderer: `render_svg(&PlotOutput, &Theme, width, height) -> String`. Handles all `Element` types, clip paths, text wrapping |
| `src/png.rs` | PNG exporter: `render_png()` — converts SVG to PNG via `resvg` with system fonts |
| `src/app.rs` | wgpu window, surface, event loop, `AppState` |
| `src/frame.rs` | Bridges `PlotOutput` to GPU — resolves layout tree, projects per-region elements through their `WindowSegment`, builds vertex/index buffers, queues text. Owns all GPU vertex types, polyline tessellation |
| `src/transform.rs` | `ContinuousNumericScale` — linear interpolation between ranges |
| `src/shader.wgsl` | WGSL shaders: general pass-through, instanced SDF points, miter-join polylines |
| `src/grammar.pest` | Pest grammar for GQL |

## Architecture

### Rendering pipeline

```
Raw data → Scale::map() → Unit::NDC        (data domain → NDC -1..1)
Blueprint::render()      → PlotOutput       (elements partitioned by RegionKey + LayoutNode tree)

GPU path (interactive):
  Frame::new()           → LayoutNode::resolve()  (layout tree + root segment → HashMap<RegionKey, WindowSegment>)
                         → WindowSegment::abs_x/y()  (region-local NDC → absolute clip space)
                         → tessellate_polyline()     (miter joins in pixel space → NDC triangle mesh)
                         → ViewUniform * position    (view transform applied in vertex shader)

SVG/PNG path (headless):
  render_svg()           → LayoutNode::resolve()  (same layout resolution)
                         → WindowSegment::px_x/y()  (region-local NDC → pixel coordinates)
                         → SVG string with clip paths per region
  render_png()           → render_svg() → resvg rasterization
```

### Layout system

- `Unit` enum: `Pixels(u32)`, `NDC(f32)`, `Percent(f32)` — polymorphic coordinate value
- `WindowSegment`: a rectangular sub-region of the window, holds NDC and pixel scales for both axes. `with_margin()` creates a sub-segment. `slice_x()`/`slice_y()` subdivide along an axis. `px_x()`/`px_y()`/`px_width()`/`px_height()` convert to pixel coordinates for SVG export.
- `LayoutNode`: tree of `Leaf(RegionKey)` and `Split { axis, children: Vec<(SizeSpec, LayoutNode)> }`. Resolved against a root `WindowSegment` to produce per-region segments.
- `PlotRegion`: `DataArea`, `XAxisGutter`, `YAxisGutter`, `Title`, `Legend`, `Caption`, `FacetLabel`, `FacetColLabel`, `FacetRowLabel`, `Spacer`
- `RegionKey`: `{ region: PlotRegion, panel: Option<usize> }` — compound key supporting both shared regions (`panel: None`) and per-panel faceted regions (`panel: Some(i)`)
- `PlotOutput`: `{ regions: HashMap<RegionKey, Vec<Element>>, layout: LayoutNode }` — returned by `Blueprint::render()`, consumed by `Frame::new()`, `render_svg()`, or `render_png()`
- All vertex positions are in NDC (clip space) by the time they reach the GPU shader; SVG export converts to pixels via `WindowSegment::px_*()`.

### Key abstraction boundary

`Blueprint::render(PlotData) -> Result<PlotOutput>` is the clean seam between the plot model (domain logic, scales, geoms) and the rendering backends. All modules above this boundary (`plot.rs`, `aesthetic.rs`, `column.rs`, `geom.rs`, `scale.rs`, `shape.rs`, `layout.rs`) are backend-agnostic — zero wgpu/winit imports. The GPU backend (`frame.rs`, `app.rs`) and SVG/PNG backend (`svg.rs`, `png.rs`) both consume `PlotOutput`. Keep it that way.

## GQL language syntax

```
MAP x=:year, y=:sales              // default mappings
GEOM POINT                         // layer with default mappings
MAP x=:year, y=:sales, color=:region  // with color segmentation
GEOM POINT

MAP x=:day, y=:price, group=:ticker, color=:ticker
GEOM LINE                          // timeseries line plot

SCALE X_CONTINUOUS
FACET WRAP :store                              // wrap panels into auto grid
FACET WRAP :store COLUMNS 3                    // wrap, forced 3 columns
FACET GRID ROWS :store                         // single column of panels
FACET GRID COLS :store                         // single row of panels
FACET GRID ROWS :store COLS :town              // cross-product matrix

// Scale controls (appended to any variant):
FACET WRAP :store SCALES FREE                  // both axes free per panel
FACET WRAP :store SCALES FREE X                // x free, y shared
FACET WRAP :store SCALES FREE Y                // y free, x shared
FACET WRAP :store SCALES FIXED                 // both shared (default)
TITLE "My plot"
```

Data variables are referenced with `:` prefix. `MAP` sets plot-level defaults; geom-level `{ }` overrides per-layer. Faceting splits data into panels; `WRAP` auto-grids one variable, `GRID` creates a strict rows×cols matrix from one or two variables. `SCALES` controls axis sharing (`FREE`/`FREE X`/`FREE Y`/`FIXED`).

## Key architectural decisions

- **Theme is borrowed, not owned** by `Blueprint` — themes affect things beyond the plot scope (window margin, background) and may be shared across multiple plots.
- **`Element` enum** (`Rect | Point | Polyline | Text`) unifies geometry at the render boundary. All variants carry domain-level data (positions in `Unit` coords); `Frame` converts them to GPU-specific formats, `render_svg()` converts to SVG elements.
- **`Mapping` is a struct** `{ aesthetic: Aesthetic, variable: String }` — extensible to any aesthetic channel. `Aesthetic` and `AestheticFamily` are enums, not traits.
- **Split data pipeline types**: `RawColumn` (input: `FloatArray`/`IntArray`/`StringArray`) and `MappedColumn` (output: `UnitArray`/`ColorArray`). `AesData` (`HashMap<Aesthetic, RawColumn>`) is produced by the column-rename step; `ResolvedData` (`mapped: HashMap<Aesthetic, MappedColumn>`, `raw: HashMap<Aesthetic, RawColumn>`) is produced by bulk scale mapping and passed to `Geometry::render()`. `PlotData` (`HashMap<String, RawColumn>`) remains the CSV boundary type. `Scale::map()` takes `&RawColumn → Result<MappedColumn>`, eliminating in-geom scale lookups.
- **Faceting uses `FacetSpec` enum** (`Wrap { variable, columns, scales }` | `Grid { row_var, col_var, scales }`), stored as `Option<FacetSpec>` on Blueprint. `ScaleFreedom` enum (`Fixed | FreeX | FreeY | Free`) controls axis sharing. `Scale::clone_unfitted()` creates blank copies for per-panel or per-row/column scale instances.
- **SVG/PNG exporters are backend-agnostic** — they consume the same `PlotOutput` as the GPU renderer and use `WindowSegment::px_*()` methods for coordinate conversion. PNG export works by rendering to SVG first, then rasterizing via `resvg`.

## Issues and project planning

Open issues in `proj/issues/`:
- `issue-error-handling.md` — In progress (basic `GglangError` infrastructure in place, further refinement ongoing)
- `issue-scale-position-discrete.md` — Not started (required for bar charts / `GeomBar`)
- `issue-scale-log.md` — Not started (log scale support)

Active work tracked in `proj/backlog.md`. Design notes and language examples in `docs/` and `proj/ideas/`.

## Running

```bash
cargo run --bin plot file.gg data.csv                       # interactive wgpu window
cargo run --bin plot file.gg data.csv --output out.svg      # export SVG
cargo run --bin plot file.gg data.csv --output out.png      # export PNG
cargo run --bin plot file.gg data.csv --output out.png --width 3200 --height 2400  # custom dimensions
cargo run -- path.gg                                        # parser (prints statement types)
```

### Verifying visual output

Use SVG/PNG export to verify rendering without launching an interactive window. This is the preferred way to check visual correctness during development:

```bash
cargo run --bin plot examples/scatter.gg examples/data.csv --output /tmp/test.svg
open /tmp/test.svg    # preview in browser/viewer
```

SVG is best for quick iteration (instant export, inspectable markup). PNG is useful for final output or when you need rasterized pixels. Default dimensions are 2400x1800; use `--width`/`--height` to adjust.
