# CLAUDE.md

This file contains instructions for Claude for working with this repo.

As you make changes, keep this file up to date with the useful context to help you and others.

./proj contains files related to project management; issues are in ./proj/issues and ./proj/stories contains user stories.
We'll work off of ./proj/roadmap.md; as you make changes, keep these up to date (delete tickets that are completed,
update roadmap doc to reflect current status).

## Project Overview

A ggplot2-inspired statistical graphics engine written in Rust, using wgpu for GPU-accelerated rendering. The system has two components: a DSL compiler for a language called GQL (Grammar of Graphics Language), and a rendering engine that produces plots from a `Blueprint` specification.

Goals:

- A declarative, language-independent visualization definition (GQL)
- High-performance native rendering suitable for large datasets and interactivity
- Grammar-of-graphics compositional model (layers, scales, aesthetics, stats, facets)
- Deviations from ggplot2: data is decoupled from the plot definition (passed at render time, not embedded); lower-level channel mappings (e.g. `width`/`height` separate from `size`); less reliance on tidy data


## Current state

The parser, compiler, and renderer are connected end-to-end. A `.gg` file and CSV are parsed, compiled into a `Blueprint`, and rendered via wgpu. Supported features: `GeomPoint` and `GeomLine` with X/Y continuous scales, `group` aesthetic for partitioning line series, color segmentation via `ScaleColorDiscrete` (categorical string column → HSL-spaced colors with legend), axis tick marks/labels, plot titles/captions/axis labels, and faceting via `FACET BY :var` (splits data into a grid of sub-plots by categorical variable, with shared scales across panels). CSV loading auto-detects numeric vs. string columns. A tree-based layout system gives each plot region (data area, axis gutters, title, legend, caption, facet labels) its own `WindowSegment`, replacing the previous out-of-bounds NDC positioning. `RegionKey` (compound `PlotRegion` + optional panel index) enables both single-plot and faceted layouts through the same rendering pipeline. The rendering backend uses three separate pipelines: a general pipeline for rectangles/axes/ticks, an instanced SDF pipeline for anti-aliased points, and a miter-join tessellated pipeline for polylines. A view transform uniform (currently identity) unblocks future pan/zoom. The domain modules (`shape.rs`, `layout.rs`, `plot.rs`) are fully backend-agnostic — all wgpu/winit types are confined to `frame.rs` and `app.rs`, enabling GPU-free testing and future SVG/PNG export.

## Module map

| File | Role |
|------|------|
| `src/main.rs` | Parser binary — reads `.gg` file, identifies statement types |
| `src/bin/plot.rs` | Renderer binary — launches wgpu window, loads `.gg` + CSV |
| `src/lib.rs` | Library root — re-exports modules |
| `src/ast.rs` | AST types and Pest parser: `Program`, `Statement`, `AstAesthetic`, `DataMapping` |
| `src/compile.rs` | Compiles AST `Program` into a `Blueprint` — wires mappings, layers, scales |
| `src/data.rs` | CSV loader — auto-detects numeric (`FloatArray`) vs. string (`StringArray`) columns |
| `src/app.rs` | wgpu window, surface, event loop, `AppState` |
| `src/frame.rs` | Bridges `PlotOutput` to GPU — resolves layout tree, projects per-region elements through their `WindowSegment`, builds vertex/index buffers, queues text. Owns all GPU vertex types (`Vertex`, `LineVertex`, `QuadVertex`, `PointInstance`), vertex generation (`rectangle_vertices`), text-to-glyph conversion (`text_to_section`), and polyline tessellation |
| `src/layout.rs` | Layout system: `Unit`, `WindowSegment` (with `slice_x`/`slice_y`), `PlotRegion`, `LayoutNode`, `SizeSpec`, `SplitAxis`, `PlotOutput`. Backend-agnostic (no wgpu/winit imports) |
| `src/plot.rs` | Core domain model: `Blueprint`, `Layer`, `Geometry` trait, `Scale` trait, `Aesthetic`/`AestheticFamily` enums, `ScalePositionContinuous`, `ScaleColorDiscrete`, `GeomPoint`, `GeomLine`, `RawColumn`, `MappedColumn`, `AesData`, `ResolvedData`, `PlotData` |
| `src/shape.rs` | Domain-level render primitives: `Rectangle`, `Text`, `PolylineData`, `PointData`, `Element` enum. Backend-agnostic (no wgpu imports) |
| `src/transform.rs` | `ContinuousNumericScale` — linear interpolation between ranges |
| `src/shader.wgsl` | WGSL shaders: general pass-through (`vs_main`/`fs_main`), instanced SDF points (`vs_point_instanced`/`fs_point`), miter-join polylines (`vs_line`/`fs_line` with `fwidth` AA) |
| `src/grammar.pest` | Pest grammar for GQL |

## Architecture

### Rendering pipeline

```
Raw data → Scale::map() → Unit::NDC        (data domain → NDC -1..1)
Blueprint::render()      → PlotOutput       (elements partitioned by RegionKey + LayoutNode tree)
Frame::new()             → LayoutNode::resolve()  (layout tree + root segment → HashMap<RegionKey, WindowSegment>)
                         → WindowSegment::abs_x/y()  (region-local NDC → absolute clip space)
                         → tessellate_polyline()     (miter joins in pixel space → NDC triangle mesh)
                         → ViewUniform * position    (view transform applied in vertex shader)
```

### Layout system

- `Unit` enum: `Pixels(u32)`, `NDC(f32)`, `Percent(f32)` — polymorphic coordinate value
- `WindowSegment`: a rectangular sub-region of the window, holds NDC and pixel scales for both axes. `with_margin()` creates a sub-segment. `slice_x()`/`slice_y()` subdivide along an axis.
- `LayoutNode`: tree of `Leaf(RegionKey)` and `Split { axis, children: Vec<(SizeSpec, LayoutNode)> }`. Resolved against a root `WindowSegment` to produce per-region segments.
- `PlotRegion`: `DataArea`, `XAxisGutter`, `YAxisGutter`, `Title`, `Legend`, `Caption`, `FacetLabel`, `Spacer`
- `RegionKey`: `{ region: PlotRegion, panel: Option<usize> }` — compound key supporting both shared regions (`panel: None`) and per-panel faceted regions (`panel: Some(i)`)
- `PlotOutput`: `{ regions: HashMap<RegionKey, Vec<Element>>, layout: LayoutNode }` — returned by `Blueprint::render()`, consumed by `Frame::new()`
- All vertex positions are in NDC (clip space) by the time they reach the shader.

### Key abstraction boundary

`Blueprint::render(PlotData) -> PlotOutput` is the clean seam between the plot model (domain logic, scales, geoms) and the rendering backend (wgpu). All modules above this boundary (`plot.rs`, `shape.rs`, `layout.rs`) are backend-agnostic — zero wgpu/winit imports. Keep it that way.

## GQL language syntax

```
MAP x=:year, y=:sales              // default mappings
GEOM POINT                         // layer with default mappings
MAP x=:year, y=:sales, color=:region  // with color segmentation
GEOM POINT

MAP x=:day, y=:price, group=:ticker, color=:ticker
GEOM LINE                          // timeseries line plot

SCALE X_CONTINUOUS
FACET BY :store                    // split into sub-plots by category
FACET BY :region COLUMNS 3         // force 3-column grid
TITLE "My plot"
```

Data variables are referenced with `:` prefix. `MAP` sets plot-level defaults; geom-level `{ }` overrides per-layer. `FACET BY` splits data into panels sharing scales.

## Key architectural decisions

- **Theme is borrowed, not owned** by `Blueprint` — themes affect things beyond the plot scope (window margin, background) and may be shared across multiple plots.
- **`Element` enum** (`Rect | Point | Polyline | Text`) unifies geometry at the render boundary. All variants carry domain-level data (positions in `Unit` coords); `Frame` converts them to GPU-specific formats (vertices, instanced quads, tessellated triangle meshes).
- **`Mapping` is a struct** `{ aesthetic: Aesthetic, variable: String }` — extensible to any aesthetic channel. `Aesthetic` and `AestheticFamily` are enums, not traits.
- **Split data pipeline types**: `RawColumn` (input: `FloatArray`/`IntArray`/`StringArray`) and `MappedColumn` (output: `UnitArray`/`ColorArray`) replace the old unified `PlotParameter`. `AesData` (`HashMap<Aesthetic, RawColumn>`) is produced by the column-rename step; `ResolvedData` (`mapped: HashMap<Aesthetic, MappedColumn>`, `raw: HashMap<Aesthetic, RawColumn>`) is produced by bulk scale mapping and passed to `Geometry::render()`. `PlotData` (`HashMap<String, RawColumn>`) remains the CSV boundary type. `Scale::map()` takes `&RawColumn → Result<MappedColumn>`, eliminating in-geom scale lookups.

## Issues and project planning

Open architectural issues are in `proj/issues/`:
- ~~`issue-layout-tree.md`~~ — ✅ Done
- ~~`issue-render-backend-abstraction.md`~~ — ✅ Done (phase 1: `shape.rs` and `layout.rs` fully backend-agnostic; GPU types live in `frame.rs`)
- ~~`issue-plotdata-typing.md`~~ — ✅ Done (`RawColumn`/`MappedColumn`/`AesData`/`ResolvedData` split; bulk mapping centralized in `Blueprint::render()`)
- ~~`issue-shader-architecture.md`~~ — ✅ Done (view transform uniform, instanced SDF points, miter-join polylines, separate pipelines)

Active work tracked in `proj/backlog.md`. Design notes and language examples in `docs/` and `proj/ideas/`.

## Running

```bash
cargo run --bin plot file.gg data.csv   # compile + render
cargo run -- path.gg                    # parser (prints statement types)
```
