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

Supported features: `GeomPoint`, `GeomLine`, `GeomBar`, `GeomText`, and `GeomHistogram` (via `GEOM HISTOGRAM`) with X/Y continuous and discrete scales, `group` aesthetic for partitioning line series, `fill` aesthetic for bar interior color (separate from `color` border), `alpha` aesthetic for transparency control (mapped via `ScaleAlphaContinuous` numeric → [0.1, 1.0] or constant per-layer), `label` aesthetic for text annotation (string or numeric column; raw values flow through `ResolvedData.raw`, no scale), `shape` aesthetic for point marker glyph (categorical string → cycling palette of 5 SDF shapes: circle/triangle/square/diamond/cross, via `ScaleShapeDiscrete`; constant `{ shape="triangle" }` per-layer), color segmentation via `ScaleColorDiscrete` (categorical string column → HSL-spaced colors with swatch legend) or `ScaleColorContinuous` (numeric column → viridis gradient with bar legend; auto-detected from column type), `StatCount` for automatic frequency counting when Y is unmapped, `StatBin` for continuous-X histogram binning (Sturges' rule default, or explicit `BINS N` modifier), position adjustments (`STACK`/`DODGE`) for bar/histogram grouping, axis tick marks/labels, plot titles/captions/axis labels. `Text` carries a `color: [f32; 4]` field rendered as SVG `fill`/`fill-opacity` and GPU section color. Faceting supports two modes: `FACET WRAP :var` (wraps panels into an auto-sized grid, one variable, configurable columns) and `FACET GRID ROWS :r COLS :c` (strict rows×cols matrix from one or two variables, with col labels top, row labels right). Both modes support scale freedom controls: `SCALES FREE/FREE X/FREE Y/FIXED`. Free scales in wrap mode are per-panel; in grid mode, free X scales are shared per column and free Y scales are shared per row. The `Scale` trait has `clone_unfitted()` for creating blank scale copies used by free-scale rendering. Polar coordinates via `COORD POLAR` transform bars into arc wedges (rose diagrams), lines into closed radar polygons, and points into polar scatter. Polar axis rendering replaces Cartesian axes with concentric circles and radial spokes inside the data area.

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
| `src/geom.rs` | `Geometry` trait, `GeomPoint`, `GeomLine`, `GeomBar`, `GeomText` — layer rendering implementations |
| `src/scale.rs` | `Scale` trait, `ScalePositionContinuous`, `ScalePositionDiscrete`, `ScaleColorDiscrete`, `ScaleColorContinuous`, `StatTransform`, `StatCount`, `default_scale_for()` |
| `src/plot.rs` | `Blueprint` (builder + render orchestration), `Layer`, `PlotData`, `FacetSpec`, `CoordinateSystem` |
| `src/shape.rs` | Domain-level render primitives: `Rectangle`, `Text`, `PolylineData`, `PointData`, `ArcData`, `GradientBarData`, `Element` enum. Backend-agnostic |
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

MAP x=:category
GEOM BAR                           // count bar chart (stat=count, auto Y)

MAP x=:year, y=:sales, fill=:region
GEOM BAR                           // stacked bar chart (default position=stack)
GEOM BAR DODGE                     // dodged bar chart
GEOM BAR STACK                     // explicit stack (same as default)

MAP x=:x, y=:y, alpha=:density    // alpha mapped to numeric variable
GEOM POINT { alpha=0.3 }          // constant alpha per layer

MAP x=:gdp, y=:life, label=:country
GEOM TEXT                          // text labels at each data point
MAP x=:x, y=:y
GEOM POINT
GEOM TEXT { label=:name, color="#444444" }  // per-layer label mapping with constant color

MAP x=:income
GEOM HISTOGRAM                     // histogram with Sturges' rule bin count
GEOM HISTOGRAM BINS 30             // explicit bin count
MAP x=:income, fill=:education
GEOM HISTOGRAM BINS 20             // stacked histogram (default position=stack)
GEOM HISTOGRAM BINS 20 DODGE       // dodged histogram

MAP x=:height, y=:weight, shape=:species
GEOM POINT                         // markers vary by species (circle/triangle/square/diamond/cross)
GEOM POINT { shape="triangle" }    // constant per-layer shape

SCALE X CONTINUOUS                             // explicit continuous X (default for numeric)
SCALE X DISCRETE                              // force categorical X (auto-detected for string columns)
SCALE Y DISCRETE                              // force categorical Y
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
COORD POLAR                                    // polar coordinate transform
COORD POLAR START 1.57                         // polar with angle offset (radians)
COORD CARTESIAN                                // explicit Cartesian (default)
TITLE "My plot"
```

Data variables are referenced with `:` prefix. `MAP` sets plot-level defaults; geom-level `{ }` overrides per-layer. `SCALE X/Y CONTINUOUS/DISCRETE` overrides auto-detected scale types — string columns auto-select `DISCRETE`, numeric columns auto-select `CONTINUOUS`. `GEOM BAR` supports optional position adjustment (`STACK`/`DODGE`) after attributes: `GEOM BAR { fill=:region } DODGE`. If Y is not mapped, `StatCount` automatically counts occurrences per x category. Faceting splits data into panels; `WRAP` auto-grids one variable, `GRID` creates a strict rows×cols matrix from one or two variables. `SCALES` controls axis sharing (`FREE`/`FREE X`/`FREE Y`/`FIXED`). `COORD POLAR` transforms the coordinate system: X maps to angle, Y maps to radius. Bars become arc wedges (rose diagrams), lines become closed polygons (radar charts), points are repositioned in polar space. Optional `START` parameter sets the angular offset in radians.

## Key architectural decisions

- **Theme is borrowed, not owned** by `Blueprint` — themes affect things beyond the plot scope (window margin, background) and may be shared across multiple plots.
- **`Element` enum** (`Rect | Point | Polyline | Text | Arc`) unifies geometry at the render boundary. All variants carry domain-level data (positions in `Unit` coords); `Frame` converts them to GPU-specific formats, `render_svg()` converts to SVG elements.
- **`Mapping` is a struct** `{ aesthetic: Aesthetic, variable: String }` — extensible to any aesthetic channel. `Aesthetic` and `AestheticFamily` are enums, not traits.
- **Split data pipeline types**: `RawColumn` (input: `FloatArray`/`IntArray`/`StringArray`) and `MappedColumn` (output: `UnitArray`/`ColorArray`/`FloatArray`). `AesData` (`HashMap<Aesthetic, RawColumn>`) is produced by the column-rename step; `ResolvedData` (`mapped: HashMap<Aesthetic, MappedColumn>`, `raw: HashMap<Aesthetic, RawColumn>`) is produced by bulk scale mapping and passed to `Geometry::render()`. `PlotData` (`HashMap<String, RawColumn>`) remains the CSV boundary type. `Scale::map()` takes `&RawColumn → Result<MappedColumn>`, eliminating in-geom scale lookups.
- **Fill vs Color aesthetics**: `Fill` controls interior color (bars), `Color` controls border/stroke. `ScaleColorDiscrete` is parameterized with an `AestheticFamily` field to serve both `Color` and `Fill` families. `default_scale_for(Aesthetic::Fill)` returns a fill-flavored instance.
- **Alpha aesthetic**: `ScaleAlphaContinuous` maps a numeric domain linearly to `[0.1, 1.0]` (matching ggplot2's default range — avoids fully invisible points). Constant alpha (`GEOM POINT { alpha=0.3 }`) bypasses the scale and injects `MappedColumn::FloatArray` directly. All geoms extract alpha via `get_alpha()` helper, defaulting to 1.0 when unmapped.
- **Shape aesthetic**: `Aesthetic::Shape` / `AestheticFamily::Shape`. `ScaleShapeDiscrete` maps a categorical string column to a cycling palette of `u32` glyph indices (`NUM_SHAPES = 5`: 0=circle, 1=triangle, 2=square, 3=diamond, 4=cross). `MappedColumn::ShapeArray(Vec<u32>)` carries the per-point indices to `GeomPoint`, which threads them into `PointData::shape`. Constants (`GEOM POINT { shape="triangle" }`) bypass the scale via `ConstantValue::Shape(u32)` (parsed by `parse_shape_name()` in `aesthetic.rs`) and inject `MappedColumn::ShapeArray` directly. GPU: `PointInstance::shape_id` is a `Uint32` instance attribute (shader_location 6); `fs_point` branches on `shape_id` to select the SDF (circle: `length(p)`; square: Chebyshev; diamond: L1; triangle: equilateral SDF; cross: union of two thin boxes via `sd_box`). SVG: `render_point_shape()` emits a `<circle>`, `<polygon>`, or pair of `<rect>`s depending on shape. Shape legend: `ScaleShapeDiscrete::render` emits black `Element::Point` swatches per category — reuses the standard point rendering path so glyphs match the data area exactly. The Legend region is included in the layout when *any* of Color/Fill/Shape scales are present (see `has_legend`).
- **Label aesthetic**: `Aesthetic::Label` / `AestheticFamily::Label` — no scale (`default_scale_for(Label) => None`). Raw string/numeric column flows through `ResolvedData.raw` to `GeomText`, which reads it as `RawColumn::StringArray` or coerces numeric via `as_f64()`. `Text` struct carries `color: [f32; 4]` (default black); SVG emits `fill`/`fill-opacity`, GPU passes to wgpu_text section. Font size is a field on `GeomText { font_size }`, set to `24.0` in the compiler.
- **StatCount transform**: When `GeomBar` has no Y mapping, the compiler assigns `StatCount` which groups by X (and optionally Fill) to produce frequency counts. `GeomBar::update_scales()` creates a Y continuous scale if none exists after stat transform, and feeds cumulative stacked totals to ensure the Y domain covers full stack height.
- **StatBin transform**: `GEOM HISTOGRAM` compiles to `GeomBar { width_factor: 1.0 } + StatBin { bins }`. `StatBin` divides the continuous X domain into N equal-width bins (Sturges' rule default: `ceil(log2(n)+1)`, or explicit `BINS N` grammar modifier), emitting FloatArray X (bin centers), FloatArray Y (counts), and optional StringArray Fill. Zero-count rows are emitted for empty bins to preserve axis continuity. Single-value data (zero variance) is handled with a synthetic 1-unit bin.
- **`GeomBar::width_factor`**: fraction of band width used per bar. Default `0.8` (padded bar charts); `1.0` for histograms (touching bars). Compiler sets this when building `GeomBar` from either `GEOM BAR` or `GEOM HISTOGRAM`.
- **Position adjustments**: `BarPosition` enum (`Stack`/`Dodge`) on `GeomBar`. Grammar: `GEOM BAR DODGE`, `GEOM HISTOGRAM BINS N DODGE`. Stacking computes NDC offsets using `ndc_per_unit` (slope of the linear Y scale mapping) to stack segments correctly in NDC space.
- **Faceting uses `FacetSpec` enum** (`Wrap { variable, columns, scales }` | `Grid { row_var, col_var, scales }`), stored as `Option<FacetSpec>` on Blueprint. `ScaleFreedom` enum (`Fixed | FreeX | FreeY | Free`) controls axis sharing. `Scale::clone_unfitted()` creates blank copies for per-panel or per-row/column scale instances.
- **Polar coordinates**: `CoordinateSystem` enum (`Cartesian`/`Polar { start_angle }`) on Blueprint. Transform is applied after `geometry.render()` produces elements but before they're added to the DataArea region. `Element::Rect` → `Element::Arc` (wedge), `Element::Polyline` points are remapped and polygon is closed, `Element::Point` positions are remapped. Polar axes (concentric circles, radial spokes, angular labels) render into DataArea instead of axis gutters. `polar_plot_layout()` omits axis gutter regions.
- **`Element::Arc` (ArcData)**: Domain-level arc/wedge primitive with center, inner/outer radius, start/end angle, color. SVG renders via `<path>` with arc commands (handles full-circle and annular cases). GPU tessellates into triangle fan (pie) or quad strip (annulus).
- **SVG/PNG exporters are backend-agnostic** — they consume the same `PlotOutput` as the GPU renderer and use `WindowSegment::px_*()` methods for coordinate conversion. PNG export works by rendering to SVG first, then rasterizing via `resvg`.

## Issues and project planning

Active work tracked in `proj/roadmap.md`. Stories in `proj/stories/`.

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

## Testing

Snapshot tests live in `tests/svg_snapshots.rs` and use [insta](https://insta.rs/). Each test renders an example `.gg` + `.csv` pair from `examples/` to SVG at 1200x900 and compares against the stored snapshot in `tests/snapshots/`.

```bash
cargo test                                  # run all snapshot tests
cargo test --test svg_snapshots scatter     # run a single test
cargo insta test --review                   # run tests and review diffs in one step
cargo insta review                          # review pending .snap.new files from a prior failed run
```

**Important:** `cargo insta review` only has work to do when a test *fails* (pending `.snap.new` file). If `cargo test` reports all passing, there are no snapshots to review — "no snapshots to review" is the expected output. Use `cargo insta test --review` if you want the combined run-then-review flow.

Adding a new snapshot test:

1. Drop the new example pair into `examples/` (e.g. `examples/pie.gg`, `examples/pie.csv`).
2. Add a `snapshot_test!(name, "file.gg", "file.csv");` line at the bottom of `tests/svg_snapshots.rs`.
3. Run `cargo test` — the first run fails and creates `tests/snapshots/svg_snapshots__name.snap.new`.
4. Run `cargo insta review` to accept it, which promotes the `.snap.new` to `.snap` (commit both the snapshot and the example files).

When rendering output changes intentionally (new features, layout tweaks), `cargo test` will fail on the affected snapshots; run `cargo insta review` to accept the new SVG output. Any unintentional diff across other snapshots is a regression — investigate before accepting.
